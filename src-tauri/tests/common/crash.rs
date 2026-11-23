//! Kill harness: re-launch the current test binary as a writer and kill it hard.
//!
//! The writer is an `#[ignore]` test that prints `committed <n>` after each
//! commit. No `[[bin]]` is needed, so `tauri build` is unaffected.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Env var that tells the writer test where to work.
pub const WRITER_DIR_ENV: &str = "DN_CRASH_WRITER_DIR";

/// When to kill the writer process.
#[derive(Clone, Copy, Debug)]
pub enum KillAt {
    /// After this many `committed` lines were reported in this run.
    AfterCommits(u64),
    /// This long after the process was spawned (even if it printed nothing).
    AfterDelay(Duration),
}

/// Upper bound for commits a writer makes in one run, so a writer whose
/// parent died (IDE stop, runner timeout) stops on its own. Far above any
/// kill point: a fast disk commits 2 000 small writes in about 1.2 s, which
/// is shorter than the longest `AfterDelay`, so the bound must leave room.
pub const MAX_WRITER_COMMITS: u64 = 20_000;

/// Kills and reaps the writer on every exit path, including a panic or an
/// early return in the parent.
struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

/// Runs `test_name` (an ignored writer test in the current binary) against
/// `dir`, kills it per `kill` and returns the last reported commit number
/// (0 if none was reported).
pub fn run_writer_until(test_name: &str, dir: &Path, kill: KillAt) -> u64 {
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            test_name,
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(WRITER_DIR_ENV, dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn writer process");
    let mut child = KillOnDrop(child);

    let stdout = child.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel::<u64>();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if let Some(n) = line
                .strip_prefix("committed ")
                .and_then(|n| n.trim().parse::<u64>().ok())
                && tx.send(n).is_err()
            {
                break;
            }
        }
    });

    let started = Instant::now();
    let mut last = 0;
    let mut seen = 0u64;
    loop {
        let wait = match kill {
            KillAt::AfterCommits(_) => Duration::from_secs(60),
            KillAt::AfterDelay(d) => d.saturating_sub(started.elapsed()),
        };
        if matches!(kill, KillAt::AfterDelay(_)) && wait.is_zero() {
            break;
        }
        match rx.recv_timeout(wait) {
            Ok(n) => {
                last = n;
                seen += 1;
                if matches!(kill, KillAt::AfterCommits(k) if seen >= k) {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if matches!(kill, KillAt::AfterCommits(_)) {
                    panic!("writer reported too few commits within 60 s");
                }
                break;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("writer exited before the kill point");
            }
        }
    }

    child.0.kill().ok();
    child.0.wait().expect("wait for killed writer");
    reader.join().ok();
    // Lines printed before the kill but not yet consumed also count.
    while let Ok(n) = rx.try_recv() {
        last = n;
    }
    last
}
