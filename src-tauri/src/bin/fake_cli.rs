//! Test-only stand-in for `glab` / `gh` (built only with the `test-support`
//! feature). `FAKE_CLI_SCENARIO` picks the behaviour. Invented data only.

use std::io::Write;
use std::time::Duration;

const GLAB_NO_NETWORK: &str = "Get \"https://gitlab.example.invalid/api/v4/user\": dial tcp: lookup gitlab.example.invalid: i/o timeout.\nrun glab auth login --hostname gitlab.example.invalid\n";
const GH_NO_NETWORK: &str = "error connecting to github.example.invalid\ncheck your internet connection or https://githubstatus.com\n";

fn echo_argv() -> String {
    let args: Vec<String> = std::env::args().skip(1).collect();
    serde_json::json!({ "argv": args }).to_string()
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
