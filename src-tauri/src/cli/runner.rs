//! The only code in the app that starts `glab` / `gh`.
//!
//! Blocking by design: callers wrap `run` in `spawn_blocking`. No shell, fixed
//! arguments from `CliCall`, no prompt, a time limit, and stderr is dropped as
//! soon as the classifier has seen it.

use std::io::Read;
use std::process::{Command, Stdio};
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
    glab: String,
    gh: String,
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
            glab: Tool::Glab.program().to_string(),
            gh: Tool::Gh.program().to_string(),
            timeout: Self::TIMEOUT,
            #[cfg(any(test, feature = "test-support"))]
            env: Vec::new(),
        }
    }

    /// Tests only: production code cannot inject a program path or environment.
    #[cfg(any(test, feature = "test-support"))]
    pub fn with_programs(
        glab: PathBuf,
        gh: PathBuf,
        timeout: Duration,
        env: Vec<(String, String)>,
    ) -> Self {
        Self {
            glab: glab.to_string_lossy().into_owned(),
            gh: gh.to_string_lossy().into_owned(),
            timeout,
            env,
        }
    }

    fn program(&self, tool: Tool) -> &str {
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
        let stdout_rx = spawn_reader(child.stdout.take(), STDOUT_CAP);
        let stderr_rx = spawn_reader(child.stderr.take(), STDERR_CAP);

        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(failed(None));
                }
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CliError::Unavailable { tool });
            }
            std::thread::sleep(POLL_INTERVAL);
        };

        // A grandchild holding the pipes open must not hang us: bounded wait.
        let stdout = stdout_rx.recv_timeout(READER_JOIN_BOUND).ok();
        let stderr = stderr_rx.recv_timeout(READER_JOIN_BOUND).ok();
        let (Some(stdout), Some(stderr)) = (stdout, stderr) else {
            return Err(failed(status.code()));
        };

        if stdout.overflow {
            return Err(failed(None));
        }
        let stderr_text = String::from_utf8_lossy(&stderr.bytes);
        classify(tool, status.code(), &stderr_text)?;
        drop(stderr_text);
        String::from_utf8(stdout.bytes).map_err(|_| failed(Some(0)))
    }
}

/// Reads a pipe to the end on its own thread, keeping at most `cap` bytes.
fn spawn_reader<R: Read + Send + 'static>(pipe: Option<R>, cap: usize) -> mpsc::Receiver<Captured> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
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
    });
    rx
}
