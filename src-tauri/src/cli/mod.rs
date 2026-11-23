//! The one place that names the external `glab` / `gh` calls the app may make.
//!
//! `CliCall` is a closed list of read-only calls with fixed arguments: a write
//! call or a string-built argument cannot be expressed. `CliError` carries the
//! tool and, for `Failed`, the exit code, never stderr or a hostname.
//! Nothing here starts a process or imports `tauri`.

pub mod classify;
pub mod runner;

use serde::Serialize;

pub use classify::classify;
pub use runner::CliRunner;

/// The two CLIs the app talks to. Serialised as `"glab"` / `"gh"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Glab,
    Gh,
}

impl Tool {
    pub fn program(self) -> &'static str {
        match self {
            Tool::Glab => "glab",
            Tool::Gh => "gh",
        }
    }
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.program())
    }
}

/// Closed list of allowed calls. Adding one is a reviewed change in this file;
/// the tests below guard that every argument list is read-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliCall {
    GlabVersion,
    GlabAuthStatus,
    GhVersion,
    GhAuthStatus,
}

impl CliCall {
    pub const ALL: &'static [CliCall] = &[
        CliCall::GlabVersion,
        CliCall::GlabAuthStatus,
        CliCall::GhVersion,
        CliCall::GhAuthStatus,
    ];

    pub fn tool(self) -> Tool {
        match self {
            CliCall::GlabVersion | CliCall::GlabAuthStatus => Tool::Glab,
            CliCall::GhVersion | CliCall::GhAuthStatus => Tool::Gh,
        }
    }

    pub fn args(self) -> &'static [&'static str] {
        match self {
            CliCall::GlabVersion => &["version"],
            CliCall::GlabAuthStatus | CliCall::GhAuthStatus => &["auth", "status"],
            CliCall::GhVersion => &["--version"],
        }
    }

    /// Stable key for logs.
    pub fn name(self) -> &'static str {
        match self {
            CliCall::GlabVersion => "glab:version",
            CliCall::GlabAuthStatus => "glab:auth-status",
            CliCall::GhVersion => "gh:version",
            CliCall::GhAuthStatus => "gh:auth-status",
        }
    }
}

/// How a call ended when it did not produce data. No variant carries stderr.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all_fields = "camelCase")]
pub enum CliError {
    #[error("{tool} is unavailable (no network or server not responding)")]
    Unavailable { tool: Tool },
    #[error("{tool} is logged out")]
    LoggedOut { tool: Tool },
    #[error("{tool} was not found")]
    ToolMissing { tool: Tool },
    #[error("{tool} failed ({})", exit_code_text(.exit_code))]
    Failed { tool: Tool, exit_code: Option<i32> },
}

fn exit_code_text(exit_code: &Option<i32>) -> String {
    match exit_code {
        Some(code) => format!("exit code {code}"),
        None => "no exit code".to_string(),
    }
}

impl CliError {
    /// Variant name, for logs.
    pub fn kind(&self) -> &'static str {
        match self {
            CliError::Unavailable { .. } => "Unavailable",
            CliError::LoggedOut { .. } => "LoggedOut",
            CliError::ToolMissing { .. } => "ToolMissing",
            CliError::Failed { .. } => "Failed",
        }
    }

    /// Fixed Polish description; never includes stderr or a hostname.
    pub fn message_pl(&self) -> String {
        match self {
            CliError::Unavailable { tool: Tool::Glab } => {
                "GitLab niedostępny (brak VPN, sieci lub serwer nie odpowiada)".to_string()
            }
            CliError::Unavailable { tool: Tool::Gh } => {
                "GitHub niedostępny (brak sieci lub serwer nie odpowiada)".to_string()
            }
            CliError::LoggedOut { tool } => {
                format!("Nie zalogowano w {tool} — uruchom {tool} auth login")
            }
            CliError::ToolMissing { tool } => format!("Nie znaleziono programu {tool}"),
            CliError::Failed {
                tool,
                exit_code: Some(code),
            } => format!("{tool} zakończył się błędem (kod {code})"),
            CliError::Failed {
                tool,
                exit_code: None,
            } => format!("{tool} zakończył się błędem (bez kodu)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VARIANT_COUNT: usize = 4;

    /// Exhaustive on purpose (no `_` arm): a new variant breaks the build here.
    fn ordinal(c: CliCall) -> usize {
        match c {
            CliCall::GlabVersion => 0,
            CliCall::GlabAuthStatus => 1,
            CliCall::GhVersion => 2,
            CliCall::GhAuthStatus => 3,
        }
    }

    #[test]
    fn all_lists_every_variant_once() {
        assert_eq!(CliCall::ALL.len(), VARIANT_COUNT);
        let mut seen = [false; VARIANT_COUNT];
        for &c in CliCall::ALL {
            let o = ordinal(c);
            assert!(o < VARIANT_COUNT, "ordinal out of range for {c:?}");
            assert!(!seen[o], "{c:?} listed twice");
            seen[o] = true;
        }
        assert!(seen.iter().all(|s| *s));
    }

    #[test]
    fn every_call_is_read_only() {
        const WRITE_FLAGS: &[&str] = &[
            "-X",
            "--method",
            "-f",
            "-F",
            "--field",
            "--raw-field",
            "--input",
            "-d",
            "--data",
        ];
        const BAD_CHARS: &str = "&|<>^%\"`;$";
        for &call in CliCall::ALL {
            let args = call.args();
            assert!(!args.is_empty(), "{call:?} has no arguments");
            let ok_shape = matches!(args[0], "version" | "--version") || args == ["auth", "status"];
            assert!(ok_shape, "{call:?} is not a known read-only shape");
            for a in args {
                assert!(!WRITE_FLAGS.contains(a), "{call:?} has write flag {a}");
                assert!(
                    !a.chars()
                        .any(|c| c.is_whitespace() || BAD_CHARS.contains(c)),
                    "{call:?} argument {a:?} has a forbidden character"
                );
            }
        }
    }

    #[test]
    fn tools_programs_and_names_are_stable() {
        assert_eq!(Tool::Glab.program(), "glab");
        assert_eq!(Tool::Gh.program(), "gh");
        assert_eq!(CliCall::GlabVersion.tool(), Tool::Glab);
        assert_eq!(CliCall::GhAuthStatus.tool(), Tool::Gh);
        assert_eq!(CliCall::GhAuthStatus.name(), "gh:auth-status");
        assert_eq!(CliCall::GlabVersion.name(), "glab:version");
    }

    #[test]
    fn failed_serializes_with_kind_and_camel_case_detail() {
        let err = CliError::Failed {
            tool: Tool::Glab,
            exit_code: Some(2),
        };
        assert_eq!(
            serde_json::to_string(&err).unwrap(),
            r#"{"kind":"Failed","detail":{"tool":"glab","exitCode":2}}"#
        );
    }

    #[test]
    fn failed_display_shows_a_plain_exit_code() {
        let failed = |exit_code| CliError::Failed {
            tool: Tool::Gh,
            exit_code,
        };
        assert_eq!(failed(Some(2)).to_string(), "gh failed (exit code 2)");
        assert_eq!(failed(None).to_string(), "gh failed (no exit code)");
    }

    #[test]
    fn kind_and_polish_messages() {
        let glab = Tool::Glab;
        let gh = Tool::Gh;
        let failed = |tool, exit_code| CliError::Failed { tool, exit_code };
        assert_eq!(CliError::Unavailable { tool: glab }.kind(), "Unavailable");
        assert_eq!(CliError::LoggedOut { tool: gh }.kind(), "LoggedOut");
        assert_eq!(CliError::ToolMissing { tool: gh }.kind(), "ToolMissing");
        assert_eq!(failed(gh, None).kind(), "Failed");
        assert_eq!(
            CliError::Unavailable { tool: glab }.message_pl(),
            "GitLab niedostępny (brak VPN, sieci lub serwer nie odpowiada)"
        );
        assert_eq!(
            CliError::Unavailable { tool: gh }.message_pl(),
            "GitHub niedostępny (brak sieci lub serwer nie odpowiada)"
        );
        assert_eq!(
            CliError::LoggedOut { tool: glab }.message_pl(),
            "Nie zalogowano w glab — uruchom glab auth login"
        );
        assert_eq!(
            CliError::ToolMissing { tool: gh }.message_pl(),
            "Nie znaleziono programu gh"
        );
        assert_eq!(
            failed(glab, Some(2)).message_pl(),
            "glab zakończył się błędem (kod 2)"
        );
        assert_eq!(
            failed(gh, None).message_pl(),
            "gh zakończył się błędem (bez kodu)"
        );
    }
}
