//! Turns the settings file and vault roots into source statuses, reusing the
//! vault guards. Nothing here imports `tauri`.

use std::path::Path;

use super::{SourceId, SourceState, SourceStatus};
use crate::settings::load_settings;
use crate::vault::{Space, VaultError, VaultRegistry};

/// One check: reads `settings.json`, builds the registry and probes every
/// source. Output order is settings, vault:work, exclusions:work,
/// vault:private, exclusions:private; omitted sources are skipped.
pub fn probe(settings_path: &Path) -> (Result<VaultRegistry, VaultError>, Vec<SourceStatus>) {
    let settings = match load_settings(settings_path) {
        Ok(s) => s,
        Err(e) => {
            let status = error_status(SourceId::Settings, "Ustawienia", &e);
            return (Err(e), vec![status]);
        }
    };
    let registry = VaultRegistry::from_settings(&settings);
    let mut sources = Vec::new();
    if settings.work.is_none() && settings.private.is_none() {
        sources.push(SourceStatus {
            id: SourceId::Settings,
            label: "Ustawienia".to_string(),
            state: SourceState::Error {
                kind: "NoSpaceConfigured".to_string(),
                message: "Nie skonfigurowano żadnego vaultu".to_string(),
            },
        });
    } else {
        sources.push(SourceStatus {
            id: SourceId::Settings,
            label: "Ustawienia".to_string(),
            state: SourceState::Ok,
        });
    }
    for space in [Space::Work, Space::Private] {
        probe_space(&registry, space, &mut sources);
    }
    (Ok(registry), sources)
}

/// The check result when the app config dir cannot be resolved, so there is no
/// `settings.json` to read. Used for the initial state and for every round.
pub fn unresolved_config_dir() -> (Result<VaultRegistry, VaultError>, Vec<SourceStatus>) {
    let e = VaultError::SettingsInvalid {
        reason: "nie można ustalić folderu konfiguracji aplikacji".to_string(),
    };
    let status = error_status(SourceId::Settings, "Ustawienia", &e);
    (Err(e), vec![status])
}

fn space_name(space: Space) -> &'static str {
    match space {
        Space::Work => "Praca",
        Space::Private => "Prywatne",
    }
}

fn probe_space(registry: &VaultRegistry, space: Space, out: &mut Vec<SourceStatus>) {
    let vault_id = SourceId::Vault(space);
    let vault_label = format!("Vault {}", space_name(space));
    let excl_id = SourceId::Exclusions(space);
    let excl_label = format!("Wykluczenia {}", space_name(space));

    let vault = match registry.vault(space) {
        Ok(v) => v,
        Err(VaultError::SpaceNotConfigured(_)) => {
            for (id, label) in [(vault_id, vault_label), (excl_id, excl_label)] {
                out.push(SourceStatus {
                    id,
                    label,
                    state: SourceState::NotConfigured,
                });
            }
            return;
        }
        Err(e) => {
            out.push(error_status(vault_id, &vault_label, &e));
            return;
        }
    };
    if let Err(e) = vault.list(None) {
        out.push(error_status(vault_id, &vault_label, &e));
        return;
    }
    out.push(SourceStatus {
        id: vault_id,
        label: vault_label,
        state: SourceState::Ok,
    });
    match vault.unmatched_exclusions() {
        Err(e) => out.push(error_status(excl_id, &excl_label, &e)),
        Ok(unmatched) if unmatched.is_empty() => out.push(SourceStatus {
            id: excl_id,
            label: excl_label,
            state: SourceState::Ok,
        }),
        Ok(unmatched) => {
            let list = unmatched
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            out.push(SourceStatus {
                id: excl_id,
                label: excl_label,
                state: SourceState::Error {
                    kind: "UnmatchedExclusions".to_string(),
                    message: format!("Wykluczenia nic nie chronią (brak w vaulcie): {list}"),
                },
            });
        }
    }
}

fn error_status(id: SourceId, label: &str, e: &VaultError) -> SourceStatus {
    SourceStatus {
        id,
        label: label.to_string(),
        state: SourceState::Error {
            kind: kind_name(e),
            message: message_pl(e),
        },
    }
}

/// The English variant name, taken from the serde tag so it covers every variant.
fn kind_name(e: &VaultError) -> String {
    serde_json::to_value(e)
        .ok()
        .and_then(|v| v.get("kind").and_then(|k| k.as_str().map(str::to_string)))
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Polish panel text for an error. May include the user's own configured path.
pub fn message_pl(e: &VaultError) -> String {
    match e {
        VaultError::NotConfigured { settings_path } => {
            format!("Brak pliku ustawień: {}", settings_path.display())
        }
        VaultError::SettingsInvalid { reason } => {
            format!("Ustawienia są nieprawidłowe: {reason}")
        }
        VaultError::RootMissing { root, .. } => {
            format!(
                "Folder vaultu nie istnieje lub jest niedostępny: {}",
                root.display()
            )
        }
        VaultError::RootsOverlap => "Foldery vaultów Praca i Prywatne nakładają się".to_string(),
        VaultError::Io { path, kind } => {
            if path.is_empty() {
                format!("Błąd odczytu: {kind}")
            } else {
                format!("Błąd odczytu ({path}): {kind}")
            }
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn message_pl_covers_listed_kinds() {
        let m = message_pl(&VaultError::NotConfigured {
            settings_path: PathBuf::from("X:/s.json"),
        });
        assert!(m.starts_with("Brak pliku ustawień: "));
        let m = message_pl(&VaultError::SettingsInvalid {
            reason: "zły".into(),
        });
        assert_eq!(m, "Ustawienia są nieprawidłowe: zły");
        let m = message_pl(&VaultError::RootMissing {
            space: Space::Work,
            root: PathBuf::from("X:/v"),
        });
        assert!(m.starts_with("Folder vaultu nie istnieje lub jest niedostępny: "));
        assert!(message_pl(&VaultError::RootsOverlap).contains("nakładają"));
        let m = message_pl(&VaultError::Io {
            path: "a.md".into(),
            kind: "PermissionDenied".into(),
        });
        assert!(m.contains("a.md") && m.contains("PermissionDenied"));
        // Fallback: Display text.
        let other = VaultError::NoFrontmatter;
        assert_eq!(message_pl(&other), other.to_string());
    }

    #[test]
    fn unresolved_config_dir_is_a_settings_error() {
        let (registry, sources) = unresolved_config_dir();
        assert!(matches!(registry, Err(VaultError::SettingsInvalid { .. })));
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, SourceId::Settings);
        match &sources[0].state {
            SourceState::Error { kind, message } => {
                assert_eq!(kind, "SettingsInvalid");
                assert!(message.starts_with("Ustawienia są nieprawidłowe: "));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn kind_name_is_the_variant_name() {
        assert_eq!(kind_name(&VaultError::RootsOverlap), "RootsOverlap");
        assert_eq!(
            kind_name(&VaultError::SettingsInvalid { reason: "x".into() }),
            "SettingsInvalid"
        );
    }
}
