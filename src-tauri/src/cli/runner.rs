//! The only code in the app that starts `glab` / `gh`.
//!
//! Blocking by design: callers wrap `run` in `spawn_blocking`. No shell, fixed
//! arguments from `CliCall`, no prompt, a time limit, and stderr is dropped as
//! soon as the classifier has seen it.
//!
//! The child runs in a fixed, neutral working directory (the system temp dir),
//! never in the app's own: both CLIs infer a repository and host from the git
//! checkout in their working directory. A repo-scoped call names its repository
//! explicitly (`--repo` / `-R`) inside its `CliCall` variant.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::{CliCall, CliError, Tool, classify};

#[cfg(any(test, feature = "test-support"))]
use std::path::PathBuf;

const STDOUT_CAP: usize = 16 * 1024 * 1024;
const STDERR_CAP: usize = 64 * 1024;
const READER_JOIN_BOUND: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Fixed environment on top of the inherited one: no colour, no prompts, no
/// update notices. The app never sets a token.
const FIXED_ENV: &[(&str, &str)] = &[
    ("NO_COLOR", "1"),
    ("CLICOLOR", "0"),
    ("GH_PROMPT_DISABLED", "1"),
    ("GH_NO_UPDATE_NOTIFIER", "1"),
    ("GH_NO_EXTENSION_UPDATE_NOTIFIER", "1"),
    ("GH_SPINNER_DISABLED", "1"),
    ("GLAB_NO_PROMPT", "true"),
    ("GLAB_CHECK_UPDATE", "false"),
    ("GLAB_NOTIFY_SKILL_UPDATES", "false"),
];

pub struct CliRunner {
    glab: OsString,
    gh: OsString,
    timeout: Duration,
    #[cfg(any(test, feature = "test-support"))]
    env: Vec<(String, String)>,
}

/// What a reader thread hands back: the kept bytes and whether the cap was hit.
struct Captured {
    bytes: Vec<u8>,
    overflow: bool,
}

impl CliRunner {
    pub const TIMEOUT: Duration = Duration::from_secs(20);

    /// Programs resolved from `PATH` by `Command::new`.
    pub fn system() -> Self {
        Self {
            glab: Tool::Glab.program().into(),
            gh: Tool::Gh.program().into(),
            timeout: Self::TIMEOUT,
            #[cfg(any(test, feature = "test-support"))]
            env: Vec::new(),
        }
    }

    /// Tests only: production code cannot inject a program path or environment.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn with_programs(
        glab: PathBuf,
        gh: PathBuf,
        timeout: Duration,
        env: Vec<(String, String)>,
    ) -> Self {
        Self {
            glab: glab.into_os_string(),
            gh: gh.into_os_string(),
            timeout,
            env,
        }
    }

    fn program(&self, tool: Tool) -> &OsStr {
        match tool {
            Tool::Glab => &self.glab,
            Tool::Gh => &self.gh,
        }
    }

    pub fn run(&self, call: CliCall) -> Result<String, CliError> {
        let started = Instant::now();
        let result = self.run_inner(call);
        let ms = started.elapsed().as_millis();
        match &result {
            Ok(_) => log::debug!("cli: {} -> data ({ms} ms)", call.name()),
            Err(e) => log::info!("cli: {} -> {} ({ms} ms)", call.name(), e.kind()),
        }
        result
    }

    fn run_inner(&self, call: CliCall) -> Result<String, CliError> {
        let tool = call.tool();
        let failed = |exit_code| CliError::Failed { tool, exit_code };

        let mut command = Command::new(self.program(tool));
        command
            .args(call.args())
            .current_dir(std::env::temp_dir())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in FIXED_ENV {
            command.env(k, v);
        }
        #[cfg(any(test, feature = "test-support"))]
        for (k, v) in &self.env {
            command.env(k, v);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(CliError::ToolMissing { tool });
            }
            Err(_) => return Err(failed(None)),
        };

        // Drain both pipes from the start so a full pipe buffer cannot block the child.
        let stdout_rx = spawn_reader("cli-stdout", child.stdout.take(), STDOUT_CAP);
        let stderr_rx = spawn_reader("cli-stderr", child.stderr.take(), STDERR_CAP);
        let (Ok(stdout_rx), Ok(stderr_rx)) = (stdout_rx, stderr_rx) else {
            stop(&mut child);
            return Err(failed(None));
        };

        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(_) => {
                    stop(&mut child);
                    return Err(failed(None));
                }
            }
            if Instant::now() >= deadline {
                stop(&mut child);
                return Err(CliError::Unavailable { tool });
            }
            std::thread::sleep(POLL_INTERVAL);
        };
        let code = status.code();

        // A grandchild holding a pipe open must not hang us: one shared bound for
        // both readers.
        let join_deadline = Instant::now() + READER_JOIN_BOUND;
        let stdout = recv_until(&stdout_rx, join_deadline);
        let Some(stderr) = recv_until(&stderr_rx, join_deadline) else {
            return Err(failed(None));
        };

        // Classify first, so a failure is named even when stdout never finished.
        let stderr_text = String::from_utf8_lossy(&stderr.bytes);
        let classified = classify(tool, code, &stderr_text);
        drop(stderr_text);
        drop(stderr);
        classified?;

        // Exit 0 from here on. Data only when stdout was read whole.
        let Some(stdout) = stdout else {
            return Err(failed(None));
        };
        if stdout.overflow {
            return Err(failed(None));
        }
        String::from_utf8(stdout.bytes).map_err(|_| failed(Some(0)))
    }
}

/// Kills the direct child and reaps it. On Windows this does not reach
/// grandchildren; see the plan's Open Risks.
fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn recv_until(rx: &mpsc::Receiver<Captured>, deadline: Instant) -> Option<Captured> {
    rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .ok()
}

/// Reads a pipe to the end on its own named thread, keeping at most `cap` bytes.
/// Fails only when the OS refuses a new thread.
fn spawn_reader<R: Read + Send + 'static>(
    name: &str,
    pipe: Option<R>,
    cap: usize,
) -> std::io::Result<mpsc::Receiver<Captured>> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            let mut captured = Captured {
                bytes: Vec::new(),
                overflow: false,
            };
            if let Some(mut pipe) = pipe {
                let mut buf = [0u8; 8192];
                loop {
                    match pipe.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let room = cap - captured.bytes.len();
                            if n > room {
                                captured.overflow = true;
                            }
                            captured.bytes.extend_from_slice(&buf[..n.min(room)]);
                        }
                    }
                }
            }
            let _ = tx.send(captured);
        })?;
    Ok(rx)
}
