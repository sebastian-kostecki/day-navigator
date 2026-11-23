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

/// Marker inside the truncated `gitlab-bad-json` body; tests assert it never
/// shows up in an error.
const BAD_JSON_MARKER: &str = "LEAK-MARKER-TITLE-7c1e";

/// Which assigned-items endpoint was asked for, from the `glab api` arguments.
#[derive(PartialEq)]
enum Endpoint {
    MergeRequests,
    Issues,
    Other,
}

fn endpoint() -> Endpoint {
    for arg in std::env::args().skip(1) {
        if arg.starts_with("merge_requests?") {
            return Endpoint::MergeRequests;
        }
        if arg.starts_with("issues?") {
            return Endpoint::Issues;
        }
    }
    Endpoint::Other
}

/// Invented assigned items on `gitlab.example.invalid`: a draft MR with labels,
/// a tie on `updated_at`, an MR with a link on a foreign host.
fn gitlab_assigned_body(endpoint: &Endpoint) -> Option<String> {
    let body = match endpoint {
        Endpoint::MergeRequests => serde_json::json!([
            {
                "title": "Fix gizmo rotation",
                "web_url": "https://other.example.invalid/acme/gizmo/-/merge_requests/7",
                "updated_at": "2026-03-01T09:00:00.000Z",
                "labels": [],
                "draft": false,
                "references": { "full": "acme/gizmo!7" }
            },
            {
                "title": "Add widget parser",
                "web_url": "https://gitlab.example.invalid/acme/widgets/-/merge_requests/34",
                "updated_at": "2026-03-02T10:00:00.000Z",
                "labels": ["backend", "review"],
                "draft": true,
                "references": { "full": "acme/widgets!34" }
            },
            {
                "title": "Rename widget module",
                "web_url": "https://gitlab.example.invalid/acme/widgets/-/merge_requests/12",
                "updated_at": "2026-03-02T10:00:00.000Z",
                "references": { "full": "acme/widgets!12" }
            }
        ]),
        Endpoint::Issues => serde_json::json!([
            {
                "title": "Gizmo jitters",
                "web_url": "https://gitlab.example.invalid/acme/gizmo/-/issues/5",
                "updated_at": "2026-02-20T08:00:00.000Z",
                "labels": ["bug"],
                "references": { "full": "acme/gizmo#5" }
            },
            {
                "title": "Widget crashes on start",
                "web_url": "https://gitlab.example.invalid/acme/widgets/-/issues/12",
                "updated_at": "2026-03-03T08:00:00.000Z",
                "labels": [],
                "references": { "full": "acme/widgets#12" }
            }
        ]),
        Endpoint::Other => return None,
    };
    Some(body.to_string())
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
        "gitlab-assigned" => match gitlab_assigned_body(&endpoint()) {
            Some(body) => {
                println!("{body}");
                0
            }
            None => 98,
        },
        "gitlab-empty" => {
            println!("[]");
            0
        }
        "gitlab-bad-json" => {
            // A truncated array that holds the marker as a title.
            println!(
                "[{{\"title\":\"{BAD_JSON_MARKER}\",\"web_url\":\"https://gitlab.example.invalid/acme/widgets\",\"updated_at\":\"2026-03"
            );
            0
        }
        "gitlab-mrs-ok-issues-no-network" => match endpoint() {
            Endpoint::MergeRequests => {
                println!("[]");
                0
            }
            _ => {
                eprint!("{GLAB_NO_NETWORK}");
                1
            }
        },
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
