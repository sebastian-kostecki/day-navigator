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

/// Marker inside the `gitlab-notes-bad-json` body; tests assert it never shows
/// up in an error.
const BAD_NOTES_MARKER: &str = "LEAK-MARKER-NOTE-4b2d";

/// Marker inside a title that parses fine (`gh-issues`, `octo-test/alpha#3`);
/// tests assert it reaches the issue list and nothing else.
const GH_TITLE_MARKER: &str = "LEAK-MARKER-GH-TITLE-5a8e";

/// The one issue that has notes in the notes scenarios: project and in-project
/// number of "Widget crashes on start". Any other issue gets an empty array.
const NOTES_PROJECT_ID: u64 = 102;
const NOTES_IID: u64 = 12;

/// Which endpoint was asked for, from the `glab api` arguments.
#[derive(PartialEq)]
enum Endpoint {
    MergeRequests,
    Issues,
    /// `merge_requests?...state=all...` (recently closed, any state).
    ClosedMergeRequests,
    /// `issues?...state=closed...` (recently closed).
    ClosedIssues,
    /// `projects/<project_id>/issues/<iid>/notes?...`
    Notes {
        project_id: u64,
        iid: u64,
    },
    Other,
}

fn parse_notes_endpoint(arg: &str) -> Option<Endpoint> {
    let rest = arg.strip_prefix("projects/")?;
    let (ids, _query) = rest.split_once("/notes?")?;
    let (project, iid) = ids.split_once("/issues/")?;
    Some(Endpoint::Notes {
        project_id: project.parse().ok()?,
        iid: iid.parse().ok()?,
    })
}

fn endpoint() -> Endpoint {
    for arg in std::env::args().skip(1) {
        // The closed calls first: they share the open calls' prefixes.
        if arg.starts_with("merge_requests?") && arg.contains("&state=all&") {
            return Endpoint::ClosedMergeRequests;
        }
        if arg.starts_with("issues?") && arg.contains("&state=closed&") {
            return Endpoint::ClosedIssues;
        }
        if arg.starts_with("merge_requests?") {
            return Endpoint::MergeRequests;
        }
        if arg.starts_with("issues?") {
            return Endpoint::Issues;
        }
        if let Some(notes) = parse_notes_endpoint(&arg) {
            return notes;
        }
    }
    Endpoint::Other
}

/// The assigned issues of the fake GitLab, in list order.
fn issue_values() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "id": 5001,
            "iid": 5,
            "project_id": 101,
            "title": "Gizmo jitters",
            "web_url": "https://gitlab.example.invalid/acme/gizmo/-/issues/5",
            "updated_at": "2026-02-20T08:00:00.000Z",
            "labels": ["bug"],
            "references": { "full": "acme/gizmo#5" }
        }),
        serde_json::json!({
            "id": 5002,
            "iid": NOTES_IID,
            "project_id": NOTES_PROJECT_ID,
            "title": "Widget crashes on start",
            "web_url": "https://gitlab.example.invalid/acme/widgets/-/issues/12",
            "updated_at": "2026-03-03T08:00:00.000Z",
            "labels": [],
            "references": { "full": "acme/widgets#12" }
        }),
    ]
}

/// One entry of the closed lists; `extra` carries the state and closing times.
fn closed_value(
    id: u64,
    iid: u64,
    project_id: u64,
    path: &str,
    reference: &str,
    title: &str,
    extra: serde_json::Value,
) -> serde_json::Value {
    let mut v = serde_json::json!({
        "id": id,
        "iid": iid,
        "project_id": project_id,
        "title": title,
        "web_url": format!("https://gitlab.example.invalid/{path}"),
        "updated_at": "2026-03-10T08:00:00.000Z",
        "labels": [],
        "references": { "full": reference }
    });
    if let (Some(base), Some(more)) = (v.as_object_mut(), extra.as_object()) {
        for (k, val) in more {
            base.insert(k.clone(), val.clone());
        }
    }
    v
}

/// What `glab api` returns for the closed calls (the server pre-filter is not
/// modelled: every item is returned and the parser decides). Tests use the
/// cutoff 2026-03-01T00:00:00Z.
fn gitlab_closed_body(endpoint: &Endpoint) -> Option<String> {
    let body = match endpoint {
        Endpoint::ClosedMergeRequests => serde_json::json!([
            closed_value(
                7001,
                40,
                102,
                "acme/widgets/-/merge_requests/40",
                "acme/widgets!40",
                "Merge widget cache",
                serde_json::json!({
                    "state": "merged",
                    "merged_at": "2026-03-05T12:00:00.000Z",
                    "closed_at": null
                })
            ),
            closed_value(
                7002,
                41,
                101,
                "acme/gizmo/-/merge_requests/41",
                "acme/gizmo!41",
                "Drop gizmo shim",
                serde_json::json!({
                    "state": "closed",
                    "closed_at": "2026-03-04T09:00:00.000Z",
                    "merged_at": null
                })
            ),
            closed_value(
                7003,
                42,
                102,
                "acme/widgets/-/merge_requests/42",
                "acme/widgets!42",
                "Still open widget change",
                serde_json::json!({ "state": "opened", "closed_at": null, "merged_at": null })
            ),
        ]),
        Endpoint::ClosedIssues => serde_json::json!([
            closed_value(
                8001,
                20,
                102,
                "acme/widgets/-/issues/20",
                "acme/widgets#20",
                "Widget cache is stale",
                serde_json::json!({ "state": "closed", "closed_at": "2026-03-06T08:00:00.000Z" })
            ),
            // Closed before the cutoff, updated (commented on) after it.
            closed_value(
                8002,
                21,
                101,
                "acme/gizmo/-/issues/21",
                "acme/gizmo#21",
                "Old gizmo complaint",
                serde_json::json!({ "state": "closed", "closed_at": "2026-02-10T08:00:00.000Z" })
            ),
        ]),
        _ => return None,
    };
    Some(body.to_string())
}

/// Invented assigned items on `gitlab.example.invalid`: a draft MR with labels,
/// a tie on `updated_at`, an MR with a link on a foreign host. With `paged`,
/// the issues come as two concatenated arrays (`[...][...]`), as `glab api
/// --paginate` prints them.
fn gitlab_assigned_body(endpoint: &Endpoint, paged: bool) -> Option<String> {
    let body = match endpoint {
        Endpoint::MergeRequests => serde_json::json!([
            {
                "id": 6001,
                "iid": 7,
                "project_id": 103,
                "title": "Fix gizmo rotation",
                "web_url": "https://other.example.invalid/acme/gizmo/-/merge_requests/7",
                "updated_at": "2026-03-01T09:00:00.000Z",
                "labels": [],
                "draft": false,
                "references": { "full": "acme/gizmo!7" }
            },
            {
                "id": 6002,
                "iid": 34,
                "project_id": 102,
                "title": "Add widget parser",
                "web_url": "https://gitlab.example.invalid/acme/widgets/-/merge_requests/34",
                "updated_at": "2026-03-02T10:00:00.000Z",
                "labels": ["backend", "review"],
                "draft": true,
                "references": { "full": "acme/widgets!34" }
            },
            {
                "id": 6003,
                "iid": 12,
                "project_id": 102,
                "title": "Rename widget module",
                "web_url": "https://gitlab.example.invalid/acme/widgets/-/merge_requests/12",
                "updated_at": "2026-03-02T10:00:00.000Z",
                "references": { "full": "acme/widgets!12" }
            }
        ]),
        Endpoint::Issues => {
            let issues = issue_values();
            if paged {
                let pages: Vec<String> = issues
                    .into_iter()
                    .map(|i| serde_json::Value::Array(vec![i]).to_string())
                    .collect();
                return Some(pages.concat());
            }
            serde_json::Value::Array(issues)
        }
        Endpoint::ClosedMergeRequests | Endpoint::ClosedIssues => {
            return gitlab_closed_body(endpoint);
        }
        Endpoint::Notes { .. } | Endpoint::Other => return None,
    };
    Some(body.to_string())
}

fn note_value(author: &str, system: bool, created_at: &str) -> serde_json::Value {
    serde_json::json!({
        "id": 9000,
        "body": "Invented comment text",
        "author": { "id": 1, "username": author, "name": "Invented Person" },
        "system": system,
        "created_at": created_at,
        "updated_at": created_at
    })
}

/// Notes for the notes scenarios, newest first. `None` when the scenario is not
/// a notes scenario or the endpoint is not a notes call. Only the fixed
/// project/iid has notes; any other issue gets `[]`.
fn gitlab_notes_body(scenario: &str, endpoint: &Endpoint) -> Option<String> {
    let Endpoint::Notes { project_id, iid } = endpoint else {
        return None;
    };
    let mine = *project_id == NOTES_PROJECT_ID && *iid == NOTES_IID;
    let notes: Vec<serde_json::Value> = match (scenario, mine) {
        (_, false) => vec![],
        // A comment by someone else, older ones by the configured user.
        ("gitlab-notes-other", true) => vec![
            note_value("other.invented", false, "2026-03-04T10:00:00.000Z"),
            note_value("invented.user", false, "2026-03-03T09:00:00.000Z"),
        ],
        // Only the configured user (one with different case) and nothing else.
        ("gitlab-notes-own", true) => vec![
            note_value("Invented.User", false, "2026-03-04T10:00:00.000Z"),
            note_value("invented.user", false, "2026-03-03T09:00:00.000Z"),
        ],
        // A system note by someone else only.
        ("gitlab-notes-system", true) => {
            vec![note_value(
                "other.invented",
                true,
                "2026-03-04T10:00:00.000Z",
            )]
        }
        ("gitlab-notes-none", true) => vec![],
        // Two pages: the qualifying comment is on the second page.
        ("gitlab-two-pages", true) => {
            let page1 = serde_json::Value::Array(vec![note_value(
                "invented.user",
                false,
                "2026-03-05T10:00:00.000Z",
            )]);
            let page2 = serde_json::Value::Array(vec![note_value(
                "other.invented",
                false,
                "2026-03-04T10:00:00.000Z",
            )]);
            return Some(format!("{page1}{page2}"));
        }
        _ => return None,
    };
    Some(serde_json::Value::Array(notes).to_string())
}

/// When `FAKE_CLI_NOTES_LOG` names a file, appends one line to it per notes
/// call, so a test can count them across processes.
fn record_notes_call() {
    let Ok(path) = std::env::var("FAKE_CLI_NOTES_LOG") else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "notes");
    }
}

/// The GitLab scenarios that serve the assigned lists and the notes call:
/// `gitlab-notes-*` and `gitlab-two-pages` (lists and notes both paged).
fn gitlab_notes_scenario(scenario: &str) -> i32 {
    let endpoint = endpoint();
    let paged = scenario == "gitlab-two-pages";
    if matches!(endpoint, Endpoint::Notes { .. }) {
        record_notes_call();
    }
    let body = match &endpoint {
        Endpoint::Notes { .. } => match scenario {
            "gitlab-notes-unavailable" => {
                eprint!("{GLAB_NO_NETWORK}");
                return 1;
            }
            // Only the issue without notes is unreachable (like a slow call that
            // timed out); the one with notes answers as in `gitlab-notes-other`.
            "gitlab-notes-one-unavailable" => {
                let Endpoint::Notes { project_id, iid } = &endpoint else {
                    return 98;
                };
                if (*project_id, *iid) != (NOTES_PROJECT_ID, NOTES_IID) {
                    eprint!("{GLAB_NO_NETWORK}");
                    return 1;
                }
                gitlab_notes_body("gitlab-notes-other", &endpoint)
            }
            "gitlab-notes-bad-json" => Some(format!(
                "[{{\"body\":\"{BAD_NOTES_MARKER}\",\"author\":{{\"username\":\"a\"}},\"system\":false,\"created_at\":\"2026-03"
            )),
            _ => gitlab_notes_body(scenario, &endpoint),
        },
        _ => gitlab_assigned_body(&endpoint, paged),
    };
    match body {
        Some(body) => {
            println!("{body}");
            0
        }
        None => 98,
    }
}

/// When `FAKE_CLI_CALL_LOG` names a file, appends the argv of every call (glab
/// and gh) as one line, so a test can see which calls were made, across
/// processes.
fn record_call() {
    let Ok(path) = std::env::var("FAKE_CLI_CALL_LOG") else {
        return;
    };
    let line = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{line}");
    }
}

/// The value after `-R` in the received arguments.
fn gh_repo_arg() -> Option<String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let at = args.iter().position(|a| a == "-R")?;
    args.get(at + 1).cloned()
}

fn gh_issue_value(repo: &str, number: u64, title: &str, updated: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "title": title,
        "url": format!("https://github.com/{repo}/issues/{number}"),
        "updatedAt": updated
    })
}

/// The `gh-issues` scenario: `gh issue list` routed on the `-R` value. Every
/// repo is invented (`octo-test/*`).
fn gh_issues_scenario() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("issue")
        || args.get(1).map(String::as_str) != Some("list")
    {
        return 98;
    }
    match gh_repo_arg().as_deref() {
        Some("octo-test/alpha") => {
            let issues = vec![
                gh_issue_value(
                    "octo-test/alpha",
                    3,
                    &format!("Gizmo jitters {GH_TITLE_MARKER}"),
                    "2026-02-20T08:00:00Z",
                ),
                gh_issue_value(
                    "octo-test/alpha",
                    11,
                    "Widget crashes on start",
                    "2026-03-03T08:00:00Z",
                ),
            ];
            println!("{}", serde_json::Value::Array(issues));
            0
        }
        Some("octo-test/beta") => {
            println!("[]");
            0
        }
        Some("octo-test/missing") => {
            eprintln!(
                "GraphQL: Could not resolve to a Repository with the name 'octo-test/missing'. (repository)"
            );
            1
        }
        Some("octo-test/garbled") => {
            println!(
                "[{{\"number\":1,\"title\":\"{BAD_JSON_MARKER}\",\"url\":\"https://github.com/octo-test/garbled/issues/1\",\"updatedAt\":\"2026-03"
            );
            0
        }
        Some("octo-test/huge") => {
            let issues: Vec<serde_json::Value> = (1..=500)
                .map(|n| {
                    gh_issue_value(
                        "octo-test/huge",
                        n,
                        "Invented issue",
                        "2026-01-01T00:00:00Z",
                    )
                })
                .collect();
            println!("{}", serde_json::Value::Array(issues));
            0
        }
        Some("octo-test/offline") => {
            eprintln!(
                "error connecting to api.github.com: Post \"https://api.github.com/graphql\": dial tcp: lookup api.github.com: no such host"
            );
            1
        }
        Some("octo-test/logged-out") => {
            eprintln!("To get started with GitHub CLI, please run:  gh auth login");
            4
        }
        _ => 98,
    }
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
    record_call();
    let code = match scenario.as_str() {
        "gh-issues" => gh_issues_scenario(),
        "data" => {
            println!("{}", echo_argv());
            0
        }
        "data-with-noise" => {
            println!("{}", echo_argv());
            eprintln!("A new release is available: 9.9.9 (update notice)");
            0
        }
        "gitlab-assigned" => match gitlab_assigned_body(&endpoint(), false) {
            Some(body) => {
                println!("{body}");
                0
            }
            None => 98,
        },
        "gitlab-notes-other"
        | "gitlab-notes-own"
        | "gitlab-notes-system"
        | "gitlab-notes-none"
        | "gitlab-notes-unavailable"
        | "gitlab-notes-one-unavailable"
        | "gitlab-notes-bad-json"
        | "gitlab-two-pages" => gitlab_notes_scenario(&scenario),
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
        // The open lists and the closed merge requests answer; the closed
        // issues call has no network.
        "gitlab-closed-issues-no-network" => match endpoint() {
            Endpoint::ClosedIssues => {
                eprint!("{GLAB_NO_NETWORK}");
                1
            }
            other => match gitlab_assigned_body(&other, false) {
                Some(body) => {
                    println!("{body}");
                    0
                }
                None => 98,
            },
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
