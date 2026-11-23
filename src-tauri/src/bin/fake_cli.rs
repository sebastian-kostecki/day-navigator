//! Test-only stand-in for `glab` / `gh` (built only with the `test-support`
//! feature). `FAKE_CLI_SCENARIO` picks the behaviour. Invented data only.

use std::io::Write;
use std::time::Duration;

const GLAB_NO_NETWORK: &str = "Get \"https://gitlab.example.invalid/api/v4/user\": dial tcp: lookup gitlab.example.invalid: i/o timeout.\nrun glab auth login --hostname gitlab.example.invalid\n";
const GH_NO_NETWORK: &str = "error connecting to github.example.invalid\ncheck your internet connection or https://status.example.invalid\n";

/// Echoes the received arguments and the working directory as JSON.
fn echo_argv() -> String {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_default();
    serde_json::json!({ "argv": args, "cwd": cwd }).to_string()
}

/// Starts a copy of this program that sleeps a few seconds while holding the
/// inherited stdout open (stderr is not shared), like a helper process that
/// outlives the CLI.
fn spawn_lingering_child() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    // Windows hands every inheritable handle to a child, whatever its stdio
    // says. Real CLIs (Go) restrict inheritance to the listed handles; clear
    // the flag on our stderr so the grandchild holds stdout only, as theirs would.
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
        // SAFETY: the handle is this process's own stderr, valid for its lifetime.
        unsafe {
            SetHandleInformation(std::io::stderr().as_raw_handle(), HANDLE_FLAG_INHERIT, 0);
        }
    }
    let _ = std::process::Command::new(exe)
        .env("FAKE_CLI_SCENARIO", "linger")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn main() {
    let scenario = std::env::var("FAKE_CLI_SCENARIO").unwrap_or_default();
    let code = match scenario.as_str() {
        "data" => {
            println!("{}", echo_argv());
            0
        }
        "data-with-noise" => {
            println!("{}", echo_argv());
            eprintln!("A new release is available: 9.9.9 (update notice)");
            0
        }
        "glab-logged-out" => {
            eprintln!(
                "{{\"message\":\"401 Unauthorized\"}}glab: 401 Unauthorized (HTTP 401) at gitlab.example.invalid"
            );
            1
        }
        "gh-logged-out" => {
            eprintln!(
                "To get started with GitHub CLI, please run:  gh auth login -h github.example.invalid"
            );
            4
        }
        "glab-no-network" => {
            eprint!("{GLAB_NO_NETWORK}");
            1
        }
        "gh-no-network" => {
            eprint!("{GH_NO_NETWORK}");
            1
        }
        "http-503" => {
            eprintln!("glab: Service Unavailable (HTTP 503) from gitlab.example.invalid");
            1
        }
        "unknown-error" => {
            eprintln!("something unexpected happened");
            2
        }
        "hang" => {
            std::thread::sleep(Duration::from_secs(5));
            if let Ok(path) = std::env::var("FAKE_CLI_MARKER") {
                let _ = std::fs::write(path, b"survived");
            }
            loop {
                std::thread::sleep(Duration::from_secs(60));
            }
        }
        "big-output" => {
            let chunk = [b'x'; 64 * 1024];
            let total = 16 * 1024 * 1024 + 1024;
            let mut out = std::io::stdout().lock();
            let mut written = 0;
            while written < total {
                let n = chunk.len().min(total - written);
                if out.write_all(&chunk[..n]).is_err() {
                    break;
                }
                written += n;
            }
            let _ = out.flush();
            0
        }
        "linger" => {
            std::thread::sleep(Duration::from_secs(5));
            0
        }
        "data-with-lingering-child" => {
            spawn_lingering_child();
            println!("{}", echo_argv());
            0
        }
        "no-network-with-lingering-child" => {
            spawn_lingering_child();
            eprint!("{GLAB_NO_NETWORK}");
            1
        }
        "non-utf8" => {
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(&[0xff, 0xfe, 0xfd, b'\n']);
            let _ = out.flush();
            0
        }
        _ => 99,
    };
    std::process::exit(code);
}
