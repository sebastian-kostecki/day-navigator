//! `settings.json` as a document: an order-preserving JSON object plus what a
//! rewrite has to keep (BOM, line endings), and the atomic write. Nothing here
//! creates a folder, and nothing here logs or carries a path or value.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::SUPPORTED_VERSION;

const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
/// The fingerprint of a block the file does not have.
const ABSENT: &str = "absent";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReadError {
    /// Not UTF-8, not a JSON object or not `version` 1: never overwritten.
    Invalid,
    Io,
}

/// The file as read. `Debug` is not derived: the object holds work data.
pub(super) struct Document {
    /// False when there is no file yet; `root` is then `{"version": 1}`.
    pub exists: bool,
    bom: bool,
    crlf: bool,
    pub root: Map<String, Value>,
}

impl Document {
    /// A missing file (or config dir) is a new document, not an error.
    pub fn read(path: &Path) -> Result<Document, ReadError> {
        match fs::read(path) {
            Ok(bytes) => Document::parse(&bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Document::new_file()),
            Err(_) => Err(ReadError::Io),
        }
    }

    fn new_file() -> Document {
        let mut root = Map::new();
        root.insert("version".to_string(), Value::from(SUPPORTED_VERSION));
        Document {
            exists: false,
            bom: false,
            crlf: false,
            root,
        }
    }

    fn parse(bytes: &[u8]) -> Result<Document, ReadError> {
        let stripped = bytes.strip_prefix(BOM);
        let text =
            std::str::from_utf8(stripped.unwrap_or(bytes)).map_err(|_| ReadError::Invalid)?;
        let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text) else {
            return Err(ReadError::Invalid);
        };
        if root.get("version").and_then(Value::as_u64) != Some(SUPPORTED_VERSION) {
            return Err(ReadError::Invalid);
        }
        Ok(Document {
            exists: true,
            bom: stripped.is_some(),
            crlf: text.contains("\r\n"),
            root,
        })
    }

    /// Puts the block in its place (a new key goes last), or removes it while
    /// keeping the order of the others.
    pub fn splice(&mut self, key: &str, block: Option<Value>) {
        match block {
            Some(block) => {
                self.root.insert(key.to_string(), block);
            }
            None => {
                self.root.shift_remove(key);
            }
        }
    }

    /// 2-space pretty JSON with a trailing newline, in the file's own BOM and
    /// line-ending style. A raw newline cannot occur inside a JSON string, so
    /// replacing them all is safe.
    fn render(&self) -> io::Result<Vec<u8>> {
        let mut text = serde_json::to_string_pretty(&self.root)?;
        text.push('\n');
        if self.crlf {
            text = text.replace('\n', "\r\n");
        }
        let mut bytes = if self.bom { BOM.to_vec() } else { Vec::new() };
        bytes.extend_from_slice(text.as_bytes());
        Ok(bytes)
    }

    /// Replaces `path` with this document in one step. The folder must exist.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        write_atomic(path, &self.render()?)
    }
}

/// An opaque mark of a block as it is on disk; never the block text itself.
pub(super) fn fingerprint(block: Option<&Value>) -> String {
    match block {
        None => ABSENT.to_string(),
        Some(block) => format!("sha256:{:x}", Sha256::digest(block.to_string().as_bytes())),
    }
}

/// `<file name>.tmp` next to the file.
fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

/// Writes a temp file in the same folder, syncs it and renames it over `path`.
/// On any failure the temp file is removed and `path` is untouched.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = temp_path(path);
    let written = File::create(&temp)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
        // The handle is closed by now, as Windows needs for the rename.
        .and_then(|()| fs::rename(&temp, path));
    if written.is_err() {
        // Best effort: the failure to report is the write itself.
        let _ = fs::remove_file(&temp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn names(dir: &Path) -> Vec<String> {
        let mut out: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        out.sort();
        out
    }

    /// Reads `bytes` as the file, applies `splices` and returns what is written.
    fn rewrite(bytes: &[u8], splices: &[(&str, Option<Value>)]) -> Vec<u8> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, bytes).unwrap();
        let mut document = Document::read(&path).unwrap();
        for (key, block) in splices {
            document.splice(key, block.clone());
        }
        document.write(&path).unwrap();
        assert_eq!(names(dir.path()), ["settings.json"]);
        fs::read(&path).unwrap()
    }

    const HAND_WRITTEN: &str = r#"{ "zeta": {"b": 1, "a": 2}, "version": 1,
        "todo": {"limit": 3}, "spaces": [1], "alpha": "x" }"#;

    #[test]
    fn unknown_keys_and_key_order_are_kept() {
        let block = Some(json!({"limit": 4}));
        let out = rewrite(HAND_WRITTEN.as_bytes(), &[("todo", block)]);
        let want = concat!(
            "{\n",
            "  \"zeta\": {\n    \"b\": 1,\n    \"a\": 2\n  },\n",
            "  \"version\": 1,\n",
            "  \"todo\": {\n    \"limit\": 4\n  },\n",
            "  \"spaces\": [\n    1\n  ],\n",
            "  \"alpha\": \"x\"\n",
            "}\n",
        );
        assert_eq!(String::from_utf8(out).unwrap(), want);
    }

    #[test]
    fn removing_a_block_in_the_middle_keeps_the_order_and_a_new_block_goes_last() {
        let out = rewrite(
            HAND_WRITTEN.as_bytes(),
            &[
                ("todo", None),
                ("notices", Some(json!({"daySummary": false}))),
            ],
        );
        let value: Value = serde_json::from_slice(&out).unwrap();
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["zeta", "version", "spaces", "alpha", "notices"]);
    }

    #[test]
    fn bom_and_crlf_are_kept() {
        let mut input = BOM.to_vec();
        input.extend_from_slice(b"{\r\n  \"version\": 1\r\n}\r\n");
        let out = rewrite(&input, &[("todo", Some(json!({"limit": 4})))]);
        let mut want = BOM.to_vec();
        want.extend_from_slice(
            b"{\r\n  \"version\": 1,\r\n  \"todo\": {\r\n    \"limit\": 4\r\n  }\r\n}\r\n",
        );
        assert_eq!(out, want);
    }

    #[test]
    fn a_file_without_bom_and_with_lf_stays_so() {
        let out = rewrite(b"{\n\"version\": 1}", &[]);
        assert_eq!(out, b"{\n  \"version\": 1\n}\n");
    }

    #[test]
    fn a_new_file_has_version_1_no_bom_and_lf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let document = Document::read(&path).unwrap();
        assert!(!document.exists);
        assert!(!path.exists());
        document.write(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"{\n  \"version\": 1\n}\n");
        assert_eq!(names(dir.path()), ["settings.json"]);
    }

    #[test]
    fn a_missing_folder_is_never_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing").join("settings.json");
        let document = Document::read(&path).unwrap();
        assert!(document.write(&path).is_err());
        assert!(names(dir.path()).is_empty());
    }

    #[test]
    fn a_failed_rename_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        // A folder where the file should be: the temp file is written, the
        // rename over it fails.
        let path = dir.path().join("settings.json");
        fs::create_dir(&path).unwrap();
        assert!(write_atomic(&path, b"{}").is_err());
        assert_eq!(names(dir.path()), ["settings.json"]);
        assert!(path.is_dir());
    }

    #[test]
    fn broken_files_are_invalid() {
        for bytes in [
            &b"{ not json"[..],
            br#"{"version":2}"#,
            br#"{"version":"1"}"#,
            b"{}",
            b"[1]",
            &[0xFF, 0xFE, 0x00],
        ] {
            assert_eq!(Document::parse(bytes).err(), Some(ReadError::Invalid));
        }
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Document::read(dir.path()).err(), Some(ReadError::Io));
    }

    #[test]
    fn fingerprints_mark_changes_without_holding_the_text() {
        let block = json!({"host": "gitlab.example.com"});
        let mark = fingerprint(Some(&block));
        assert!(!mark.contains("gitlab"));
        assert_eq!(mark, fingerprint(Some(&block.clone())));
        let other = json!({"host": "other.example.com"});
        assert_ne!(mark, fingerprint(Some(&other)));
        assert_ne!(fingerprint(Some(&Value::Null)), fingerprint(None));
        assert_eq!(fingerprint(None), ABSENT);
    }
}
