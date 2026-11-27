//! The refresh mechanics GitLab and GitHub share: the snapshot cell (lock, wake
//! channel, ordered publication), the thread start with a `ThreadDown`
//! stand-in, the coalescing wait loop, the "refreshing" flip, the round tail
//! and the "To do" reconcile logging.
//!
//! This is the tauri side. The rules stay in `gitlab/` and `github/` and free
//! of `tauri`; the fetch and the source-only steps stay in `gitlab_refresh.rs`
//! and `github_refresh.rs`, which call this module from their own round.
//!
//! Locking rule: the snapshot lock is held only for a clone or a store, never
//! across `emit`, a tray call or a process. `published` is the one lock held
//! across the emit, because that keeps events ordered by `seq`.
//!
//! Logging: every log call passes the source's own target (`module_path!()` of
//! its module), so the log file shows the same targets as before the
//! skeleton. Text carries state kinds and elapsed time only.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Mutex, RwLock, RwLockWriteGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::runtime::{HealthState, now_ms};
use crate::todo;

/// What differs between sources in the shared mechanics: names and the log
/// target.
#[derive(Clone, Copy)]
pub(crate) struct SourceId {
    /// Event the snapshot is pushed on, e.g. the GitHub changed event.
    pub(crate) event: &'static str,
    /// Name of the refresh thread.
    pub(crate) thread: &'static str,
    /// Prefix of the log text.
    pub(crate) prefix: &'static str,
    /// Log target: `module_path!()` of the source module.
    pub(crate) target: &'static str,
}

/// A source snapshot the cell can publish.
pub(crate) trait RefreshSnapshot: Clone + Serialize + Send + Sync + 'static {
    /// Monotonic counter, bumped on every stored change.
    fn seq(&self) -> u64;
    /// Shows "refreshing" first: `refreshing = true`, `seq += 1`.
    fn begin_refresh(&mut self);
}

/// The snapshot of one source with its wake channel and publication mark.
pub(crate) struct SourceCell<S> {
    snapshot: RwLock<S>,
    /// Wakes the refresh thread for an immediate round.
    wake: mpsc::Sender<()>,
    /// `seq` of the last snapshot pushed to the webview.
    published: Mutex<u64>,
    id: SourceId,
}

impl<S: RefreshSnapshot> SourceCell<S> {
    pub(crate) fn new(starting: S, id: SourceId) -> (Self, mpsc::Receiver<()>) {
        let (wake, rx) = mpsc::channel();
        let cell = SourceCell {
            snapshot: RwLock::new(starting),
            wake,
            published: Mutex::new(0),
            id,
        };
        (cell, rx)
    }

    /// The stored snapshot: a clone under a short lock.
    pub(crate) fn snapshot(&self) -> S {
        self.snapshot
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    /// Asks the refresh thread for a round. Bursts coalesce; fails only when
    /// the thread is gone.
    pub(crate) fn wake(&self) -> Result<(), mpsc::SendError<()>> {
        self.wake.send(())
    }

    pub(crate) fn write(&self) -> RwLockWriteGuard<'_, S> {
        self.snapshot.write().unwrap_or_else(|p| p.into_inner())
    }

    /// Pushes the stored snapshot to the webview unless a newer one was already
    /// pushed.
    pub(crate) fn publish(&self, app: &AppHandle) {
        self.publish_with(|snap| {
            if let Err(e) = app.emit(self.id.event, snap) {
                log::warn!(target: self.id.target, "{}: emit failed: {e}", self.id.prefix);
            }
        });
    }

    /// Re-reads the snapshot under `published` and runs `emit` when its `seq`
    /// is newer than the last one pushed. Holding `published` across `emit`
    /// keeps events ordered by `seq`.
    fn publish_with(&self, emit: impl FnOnce(&S)) {
        let mut last = self.published.lock().unwrap_or_else(|p| p.into_inner());
        let snap = self.snapshot();
        if snap.seq() <= *last {
            return;
        }
        *last = snap.seq();
        emit(&snap);
    }

    /// Shows "refreshing" first; the stored data and state stay as they were.
    pub(crate) fn begin_round(&self, app: &AppHandle) {
        {
            let mut guard = self.write();
            guard.begin_refresh();
        }
        self.publish(app);
    }

    /// Spawns the refresh thread. When that fails, the stand-in from
    /// `thread_down` is stored and published, so the view never stays on
    /// "Pobieram…" and the tray never lacks the source, and the health round
    /// is woken.
    pub(crate) fn start_thread(
        &self,
        app: &AppHandle,
        body: impl FnOnce() + Send + 'static,
        thread_down: impl FnOnce(&S) -> S,
    ) {
        if let Err(e) = std::thread::Builder::new()
            .name(self.id.thread.into())
            .spawn(body)
        {
            log::error!(target: self.id.target, "{}: cannot start refresh thread: {e}", self.id.prefix);
            let next = thread_down(&self.snapshot());
            *self.write() = next;
            self.publish(app);
            wake_health(app);
        }
    }
}

/// Runs a health round now instead of waiting for the tray to catch up.
fn wake_health(app: &AppHandle) {
    let _ = app.state::<HealthState>().wake.send(());
}

impl SourceId {
    /// Tells the webview and the notices that "To do" rows changed.
    pub(crate) fn notify_todo_changed(&self, app: &AppHandle, changed: bool) {
        if changed && let Err(e) = app.emit(todo::CHANGED_EVENT, ()) {
            log::warn!(target: self.target, "todo: emit failed: {e}");
        }
        if changed {
            crate::events::commands::notify(app);
        }
    }

    /// The round tail: the state transition, the elapsed time and, when the
    /// source status changed, a health round now.
    pub(crate) fn end_round(
        &self,
        app: &AppHandle,
        from: &str,
        to: &str,
        started: Instant,
        status_changed: bool,
    ) {
        if from != to {
            log::info!(target: self.target, "{}: {from} -> {to}", self.prefix);
        }
        log::debug!(target: self.target, "{}: round {to} in {} ms", self.prefix, started.elapsed().as_millis());
        if status_changed {
            wake_health(app);
        }
    }

    /// Logs the result of a "To do" reconcile by kind and returns whether at
    /// least one row changed. A store error changes nothing else.
    pub(crate) fn log_reconcile(&self, result: Result<usize, todo::TodoError>) -> bool {
        match result {
            Ok(changed) => {
                if changed > 0 {
                    log::debug!(target: self.target, "todo: reconcile changed {changed} rows");
                }
                changed > 0
            }
            // An unopenable store is reported by the health round; a warning
            // every round would only repeat it.
            Err(e @ todo::TodoError::Store(_)) => {
                log::debug!(target: self.target, "todo: reconcile failed: {}", e.kind());
                false
            }
            Err(e) => {
                log::warn!(target: self.target, "todo: reconcile failed: {}", e.kind());
                false
            }
        }
    }
}

/// The `now_ms` instant of the next try after waiting `wait`.
pub(crate) fn next_refresh_at_ms(wait: Duration) -> u64 {
    now_ms().saturating_add(wait.as_millis() as u64)
}

/// Runs `round`, then waits for its returned interval or a wake. Queued wakes
/// are drained, so a burst costs one round. Returns when the wake sender is
/// gone.
pub(crate) fn run_loop(wake_rx: mpsc::Receiver<()>, mut round: impl FnMut() -> Duration) {
    loop {
        let wait = round();
        match wake_rx.recv_timeout(wait) {
            Ok(()) => while wake_rx.try_recv().is_ok() {},
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::GithubSnapshot;
    use crate::gitlab::GitlabSnapshot;
    use std::cell::{Cell, RefCell};

    const TEST_ID: SourceId = SourceId {
        event: "test-changed",
        thread: "test-refresh",
        prefix: "test",
        target: "test_target",
    };

    #[derive(Clone, Serialize)]
    struct Fake {
        seq: u64,
    }

    impl RefreshSnapshot for Fake {
        fn seq(&self) -> u64 {
            self.seq
        }
        fn begin_refresh(&mut self) {
            self.seq += 1;
        }
    }

    #[test]
    fn run_loop_coalesces_queued_wakes() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..3 {
            tx.send(()).unwrap();
        }
        let tx = RefCell::new(Some(tx));
        let calls = Cell::new(0);
        run_loop(rx, || {
            calls.set(calls.get() + 1);
            assert!(calls.get() <= 10, "run_loop did not stop");
            if calls.get() == 2 {
                tx.borrow_mut().take();
            }
            Duration::from_secs(3600)
        });
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn run_loop_loops_on_timeout() {
        let (tx, rx) = mpsc::channel::<()>();
        let tx = RefCell::new(Some(tx));
        let calls = Cell::new(0);
        run_loop(rx, || {
            calls.set(calls.get() + 1);
            assert!(calls.get() <= 10, "run_loop did not stop");
            if calls.get() == 3 {
                tx.borrow_mut().take();
            }
            Duration::from_millis(1)
        });
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn publish_with_emits_only_a_newer_seq() {
        let (cell, _rx) = SourceCell::new(Fake { seq: 0 }, TEST_ID);
        let emitted = RefCell::new(Vec::new());
        let publish = || cell.publish_with(|s| emitted.borrow_mut().push(s.seq));

        publish();
        assert!(emitted.borrow().is_empty());

        cell.write().begin_refresh();
        publish();
        assert_eq!(*emitted.borrow(), vec![1]);

        publish();
        assert_eq!(emitted.borrow().len(), 1);

        cell.write().seq = 0;
        publish();
        assert_eq!(emitted.borrow().len(), 1);
    }

    #[test]
    fn begin_refresh_marks_a_github_snapshot_refreshing() {
        let mut snap = GithubSnapshot::starting();
        snap.begin_refresh();
        assert!(snap.refreshing);
        assert_eq!(snap.seq, 1);
    }

    #[test]
    fn begin_refresh_marks_a_gitlab_snapshot_refreshing() {
        let mut snap = GitlabSnapshot::starting();
        snap.begin_refresh();
        assert!(snap.refreshing);
        assert_eq!(snap.seq, 1);
    }

    #[test]
    fn next_refresh_at_ms_is_now_plus_the_wait() {
        let before = now_ms() + 90_000;
        let got = next_refresh_at_ms(Duration::from_secs(90));
        let after = now_ms() + 90_000;
        assert!((before..=after).contains(&got));
    }
}
