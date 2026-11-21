//! Guarded access to Obsidian vaults. Nothing here imports `tauri`.

pub mod edit;
// The write layer takes raw absolute paths with no guard: only `Vault` may call
// it. Integration tests reach it through the `test-support` feature.
mod error;
#[cfg(feature = "test-support")]
pub mod fsops;
#[cfg(not(feature = "test-support"))]
mod fsops;
pub mod markdown;
mod path;

use std::fs;
use std::path::{Component, Path, PathBuf, Prefix};

pub use edit::FieldValue;
pub use error::VaultError;
pub use markdown::{Eol, Heading, Span, SpanHash};
pub use path::VaultPath;

use crate::settings::{Settings, SpaceSettings};

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Space {
    Work,
    Private,
}

impl std::fmt::Display for Space {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Space::Work => "work",
            Space::Private => "private",
        })
    }
}

/// Configured vaults, one per space. Built once from settings.
#[derive(Debug, Clone)]
pub struct VaultRegistry {
    work: Option<SpaceSettings>,
    private: Option<SpaceSettings>,
}

impl VaultRegistry {
    pub fn from_settings(settings: &Settings) -> VaultRegistry {
        VaultRegistry {
            work: settings.work.clone(),
            private: settings.private.clone(),
        }
    }

    fn config(&self, space: Space) -> Option<&SpaceSettings> {
        match space {
            Space::Work => self.work.as_ref(),
            Space::Private => self.private.as_ref(),
        }
    }

    pub fn vault(&self, space: Space) -> Result<Vault, VaultError> {
        let cfg = self
            .config(space)
            .ok_or(VaultError::SpaceNotConfigured(space))?;
        let root = canonical_root(space, &cfg.vault)?;
        let other = match space {
            Space::Work => Space::Private,
            Space::Private => Space::Work,
        };
        if let Some(other_cfg) = self.config(other) {
            if let Ok(other_root) = canonical_root(other, &other_cfg.vault) {
                if relative_remainder(&root, &other_root).is_some()
                    || relative_remainder(&other_root, &root).is_some()
                {
                    return Err(VaultError::RootsOverlap);
                }
            }
        }
        Ok(Vault {
            space,
            root: cfg.vault.clone(),
            excluded: cfg.excluded.clone(),
        })
    }
}

/// One space's vault: a configured root plus its exclusions.
#[derive(Debug, Clone)]
pub struct Vault {
    space: Space,
    root: PathBuf,
    excluded: Vec<VaultPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: VaultPath,
    pub kind: EntryKind,
}

/// A note read from disk: vault path plus raw bytes (BOM included).
#[derive(Debug, Clone)]
pub struct Note {
    path: VaultPath,
    bytes: Vec<u8>,
}

impl Note {
    pub fn path(&self) -> &VaultPath {
        &self.path
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The note text without a leading BOM.
    pub fn text(&self) -> &str {
        let body = self.bytes.strip_prefix(&BOM[..]).unwrap_or(&self.bytes);
        std::str::from_utf8(body).unwrap_or("")
    }

    /// A frontmatter field block and its hash. The hash is what a later
    /// `replace_field` expects.
    pub fn field(&self, key: &str) -> Result<FieldView, VaultError> {
        let span = markdown::find_field(&self.bytes, key)?;
        Ok(FieldView {
            raw_block: String::from_utf8_lossy(span.slice(&self.bytes)).into_owned(),
            hash: markdown::span_hash(&self.bytes, span),
        })
    }

    /// A heading section body and its hash. The hash is what a later
    /// `replace_section` expects.
    pub fn section(&self, heading: &Heading) -> Result<SectionView, VaultError> {
        let span = markdown::find_section(&self.bytes, heading)?;
        Ok(SectionView {
            body: String::from_utf8_lossy(span.slice(&self.bytes)).into_owned(),
            hash: markdown::span_hash(&self.bytes, span),
        })
    }
}

/// A frontmatter field as read: `raw_block` is the raw block text (the
/// `key:` line plus any list lines, with their original line endings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldView {
    pub raw_block: String,
    pub hash: SpanHash,
}

/// A section body as read, with its original line endings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionView {
    pub body: String,
    pub hash: SpanHash,
}

impl Vault {
    pub fn space(&self) -> Space {
        self.space
    }

    fn is_excluded(&self, path: &VaultPath) -> bool {
        self.excluded.iter().any(|e| path.starts_with_ci(e))
    }

    /// Guard: turns a vault path into a checked absolute path, or a typed error.
    pub(crate) fn resolve(&self, path: &VaultPath) -> Result<PathBuf, VaultError> {
        if self.is_excluded(path) {
            return Err(VaultError::Excluded {
                path: path.to_string(),
            });
        }
        let root = canonical_root(self.space, &self.root)?;
        let mut joined = root.clone();
        for seg in path.segments() {
            joined.push(seg);
        }
        let canon = canonicalize_lenient(&joined, path)?;
        let remainder =
            relative_remainder(&canon, &root).ok_or_else(|| VaultError::OutsideVault {
                path: path.to_string(),
            })?;
        if remainder.iter().any(|s| s.starts_with('.')) {
            return Err(VaultError::Hidden {
                path: path.to_string(),
            });
        }
        if self.is_excluded(&VaultPath::from_segments_unchecked(remainder)) {
            return Err(VaultError::Excluded {
                path: path.to_string(),
            });
        }
        Ok(canon)
    }

    pub fn read_note(&self, path: &VaultPath) -> Result<Note, VaultError> {
        if !path.is_markdown() {
            return Err(VaultError::NotMarkdown {
                path: path.to_string(),
            });
        }
        let abs = self.resolve(path)?;
        match fs::metadata(&abs) {
            Ok(m) if m.is_file() => {}
            Ok(_) => {
                return Err(VaultError::NotFound {
                    path: path.to_string(),
                });
            }
            Err(e) => return Err(io_error(path, &e)),
        }
        let bytes = fs::read(&abs).map_err(|e| io_error(path, &e))?;
        let body = bytes.strip_prefix(&BOM[..]).unwrap_or(&bytes);
        if std::str::from_utf8(body).is_err() {
            return Err(VaultError::NotUtf8 {
                path: path.to_string(),
            });
        }
        Ok(Note {
            path: path.clone(),
            bytes,
        })
    }

    /// Non-recursive listing of notes and folders, sorted by name.
    pub fn list(&self, dir: Option<&VaultPath>) -> Result<Vec<Entry>, VaultError> {
        let display = dir.map(|d| d.to_string()).unwrap_or_default();
        let abs_dir = match dir {
            Some(d) => self.resolve(d)?,
            None => canonical_root(self.space, &self.root)?,
        };
        let io = |e: std::io::Error| VaultError::Io {
            path: display.clone(),
            kind: format!("{:?}", e.kind()),
        };
        match fs::metadata(&abs_dir) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => return Err(VaultError::NotFound { path: display }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(VaultError::NotFound { path: display });
            }
            Err(e) => return Err(io(e)),
        }
        let mut entries = Vec::new();
        for item in fs::read_dir(&abs_dir).map_err(io)? {
            let item = item.map_err(io)?;
            let Some(name) = item.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let child = match dir {
                Some(d) => d.join(&name),
                None => VaultPath::parse(&name),
            };
            let Ok(child) = child else { continue };
            let abs = match self.resolve(&child) {
                Ok(abs) => abs,
                Err(
                    VaultError::Excluded { .. }
                    | VaultError::Hidden { .. }
                    | VaultError::OutsideVault { .. },
                ) => continue,
                Err(e) => return Err(e),
            };
            let meta = match fs::metadata(&abs) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(io_error(&child, &e)),
            };
            let kind = if meta.is_dir() {
                EntryKind::Dir
            } else if meta.is_file() && child.is_markdown() {
                EntryKind::Note
            } else {
                continue;
            };
            entries.push(Entry {
                name,
                path: child,
                kind,
            });
        }
        entries.sort_by_key(|e| (e.name.to_lowercase(), e.name.clone()));
        Ok(entries)
    }

    /// Excluded entries that match nothing on disk, e.g. a typo in settings.
    /// They protect nothing, so callers surface them instead of trusting the
    /// exclusion. Only the folders leading to an entry are listed; nothing
    /// inside an excluded entry is read.
    pub fn unmatched_exclusions(&self) -> Result<Vec<VaultPath>, VaultError> {
        let root = canonical_root(self.space, &self.root)?;
        Ok(self
            .excluded
            .iter()
            .filter(|e| !exists_matching(&root, e.segments()))
            .cloned()
            .collect())
    }
}

/// True if `segs` names an existing entry under `dir`, matching each segment
/// the way exclusions match (NFC, case-insensitive).
fn exists_matching(dir: &Path, segs: &[String]) -> bool {
    let Some((first, rest)) = segs.split_first() else {
        return true;
    };
    let Ok(rd) = fs::read_dir(dir) else {
        return false;
    };
    rd.flatten().any(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| path::segment_eq(name, first))
            && (rest.is_empty() || exists_matching(&entry.path(), rest))
    })
}

const MAX_WRITE_ATTEMPTS: usize = 3;

impl Vault {
    fn ensure_root(&self) -> Result<PathBuf, VaultError> {
        canonical_root(self.space, &self.root)
    }

    /// Test hook for `apply`: lets integration tests inject an arbitrary edit.
    /// Absent from production builds, where only span-scoped edits exist.
    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn apply_for_tests(
        &self,
        path: &VaultPath,
        edit: &dyn Fn(&[u8]) -> Result<Vec<u8>, VaultError>,
    ) -> Result<(), VaultError> {
        self.apply(path, edit)
    }

    /// Guard, read, pure edit, atomic replace. Recomputes from fresh bytes when
    /// the file changed in between, up to 3 attempts, then `Conflict`.
    fn apply(
        &self,
        path: &VaultPath,
        edit: &dyn Fn(&[u8]) -> Result<Vec<u8>, VaultError>,
    ) -> Result<(), VaultError> {
        self.ensure_root()?;
        let label = path.to_string();
        for _ in 0..MAX_WRITE_ATTEMPTS {
            let note = self.read_note(path)?;
            let new_bytes = edit(note.bytes()).map_err(|e| match e {
                VaultError::Conflict { .. } => VaultError::Conflict {
                    path: label.clone(),
                },
                other => other,
            })?;
            let abs = self.resolve(path)?;
            match fsops::replace_atomic(&abs, &new_bytes, note.bytes(), &label) {
                Ok(()) => return Ok(()),
                Err(fsops::ReplaceError::Changed) => continue,
                Err(fsops::ReplaceError::Failed(e)) => return Err(e),
            }
        }
        Err(VaultError::Conflict { path: label })
    }

    pub fn replace_field(
        &self,
        path: &VaultPath,
        key: &str,
        value: FieldValue,
        expected: SpanHash,
    ) -> Result<(), VaultError> {
        self.apply(path, &|b| edit::replace_field(b, key, &value, &expected))
    }

    /// Several fields in one atomic, hash-checked write.
    pub fn replace_fields(
        &self,
        path: &VaultPath,
        edits: &[(&str, FieldValue, SpanHash)],
    ) -> Result<(), VaultError> {
        self.apply(path, &|b| edit::replace_fields(b, edits))
    }

    pub fn replace_section(
        &self,
        path: &VaultPath,
        heading: &Heading,
        body: &str,
        expected: SpanHash,
    ) -> Result<(), VaultError> {
        self.apply(path, &|b| {
            edit::replace_section(b, heading, body, &expected)
        })
    }

    pub fn append_to_section(
        &self,
        path: &VaultPath,
        heading: &Heading,
        lines: &str,
    ) -> Result<(), VaultError> {
        self.apply(path, &|b| edit::append_to_section(b, heading, lines))
    }

    /// Creates a new note; missing parent folders are created under the existing root only.
    pub fn create_note(&self, path: &VaultPath, content: &str) -> Result<(), VaultError> {
        self.create_note_with(path, content, true)
    }

    /// Creates a new note in a folder that must already exist. A missing parent
    /// is `NotFound` naming that folder, and nothing is created. Use it for
    /// configured folders, which the app never creates.
    pub fn create_note_in_existing_folder(
        &self,
        path: &VaultPath,
        content: &str,
    ) -> Result<(), VaultError> {
        self.create_note_with(path, content, false)
    }

    fn create_note_with(
        &self,
        path: &VaultPath,
        content: &str,
        make_parents: bool,
    ) -> Result<(), VaultError> {
        self.ensure_root()?;
        let label = path.to_string();
        if !path.is_markdown() {
            return Err(VaultError::NotMarkdown { path: label });
        }
        let abs = self.resolve(path)?;
        if abs.exists() {
            return Err(VaultError::AlreadyExists { path: label });
        }
        if let Some(parent) = abs.parent() {
            if make_parents {
                fs::create_dir_all(parent).map_err(|e| io_error(path, &e))?;
            } else if !parent.is_dir() {
                let segs = path.segments();
                return Err(VaultError::NotFound {
                    path: segs[..segs.len() - 1].join("/"),
                });
            }
        }
        // Folders now exist: re-run containment and exclusion before writing.
        let abs = self.resolve(path)?;
        fsops::create_new(&abs, content.as_bytes(), &label)
    }

    /// Moves a note into the root-level `.trash/` and returns its name there.
    pub fn trash(&self, path: &VaultPath) -> Result<String, VaultError> {
        let root = self.ensure_root()?;
        let label = path.to_string();
        if !path.is_markdown() {
            return Err(VaultError::NotMarkdown { path: label });
        }
        let abs = self.resolve(path)?;
        match fs::metadata(&abs) {
            Ok(m) if m.is_file() => {}
            Ok(_) => return Err(VaultError::NotFound { path: label }),
            Err(e) => return Err(io_error(path, &e)),
        }
        let trash_dir = trash_dest(&root)?;
        let file_name = path.file_name().unwrap_or_default().to_string();
        let (stem, ext) = match file_name.rfind('.') {
            Some(i) => (&file_name[..i], &file_name[i..]),
            None => (file_name.as_str(), ""),
        };
        let mut n = 1u32;
        loop {
            let candidate = if n == 1 {
                file_name.clone()
            } else {
                format!("{stem} {n}{ext}")
            };
            match fsops::move_noclobber(&abs, &trash_dir.join(&candidate), &label) {
                Ok(()) => return Ok(candidate),
                Err(VaultError::AlreadyExists { .. }) if n < 10_000 => n += 1,
                Err(e) => return Err(e),
            }
        }
    }
}

/// The only code that names `.trash`: the root-level folder, created if
/// missing, canonicalised and required to stay exactly `<root>/.trash`
/// (a junction pointing elsewhere is refused).
fn trash_dest(root: &Path) -> Result<PathBuf, VaultError> {
    let outside = || VaultError::OutsideVault {
        path: ".trash".to_string(),
    };
    let dir = root.join(".trash");
    match fs::create_dir(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => {
            return Err(VaultError::Io {
                path: ".trash".to_string(),
                kind: format!("{:?}", e.kind()),
            });
        }
    }
    let canon = dunce::canonicalize(&dir).map_err(|_| outside())?;
    match relative_remainder(&canon, root) {
        Some(rest)
            if rest.len() == 1 && rest[0].eq_ignore_ascii_case(".trash") && canon.is_dir() =>
        {
            Ok(canon)
        }
        _ => Err(outside()),
    }
}

fn io_error(path: &VaultPath, e: &std::io::Error) -> VaultError {
    if e.kind() == std::io::ErrorKind::NotFound {
        VaultError::NotFound {
            path: path.to_string(),
        }
    } else {
        VaultError::Io {
            path: path.to_string(),
            kind: format!("{:?}", e.kind()),
        }
    }
}

fn canonical_root(space: Space, root: &Path) -> Result<PathBuf, VaultError> {
    match dunce::canonicalize(root) {
        Ok(p) if p.is_dir() => Ok(p),
        _ => Err(VaultError::RootMissing {
            space,
            root: root.to_path_buf(),
        }),
    }
}

/// Canonicalises `p`, or its deepest existing ancestor with the missing tail re-appended.
fn canonicalize_lenient(p: &Path, vp: &VaultPath) -> Result<PathBuf, VaultError> {
    let mut existing = p.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        match dunce::canonicalize(&existing) {
            Ok(mut canon) => {
                for part in tail.iter().rev() {
                    canon.push(part);
                }
                return Ok(canon);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let Some(name) = existing.file_name().map(|n| n.to_os_string()) else {
                    return Err(io_error(vp, &e));
                };
                tail.push(name);
                if !existing.pop() {
                    return Err(io_error(vp, &e));
                }
            }
            Err(e) => return Err(io_error(vp, &e)),
        }
    }
}

/// Case-insensitive comparison key. Verbatim prefixes compare equal to their
/// plain forms: `dunce` keeps `\\?\` on paths longer than 260 characters, so a
/// deep note canonicalises to `\\?\D:\…` while its root stays `D:\…`.
fn component_key(c: Component<'_>) -> String {
    if let Component::Prefix(p) = c {
        match p.kind() {
            Prefix::Disk(d) | Prefix::VerbatimDisk(d) => {
                return format!("{}:", (d as char).to_ascii_lowercase());
            }
            Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => {
                return format!(
                    r"\\{}\{}",
                    server.to_string_lossy(),
                    share.to_string_lossy()
                )
                .to_lowercase();
            }
            _ => {}
        }
    }
    c.as_os_str().to_string_lossy().to_lowercase()
}

/// If `path` is `base` or lies under it (case-insensitive, whole components),
/// returns the remaining components.
fn relative_remainder(path: &Path, base: &Path) -> Option<Vec<String>> {
    let mut p = path.components();
    for b in base.components() {
        let a = p.next()?;
        if component_key(a) != component_key(b) {
            return None;
        }
    }
    Some(
        p.map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(bytes: &[u8]) -> Note {
        Note {
            path: VaultPath::parse("a.md").unwrap(),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn note_views_expose_block_text_and_matching_hashes() {
        let n = note(b"---\nStatus:\n  - Open\n---\n## Plan\nbody\n");
        let f = n.field("Status").unwrap();
        assert_eq!(f.raw_block, "Status:\n  - Open\n");
        let s = n.section(&Heading::new(2, "Plan")).unwrap();
        assert_eq!(s.body, "body\n");
        let out =
            edit::replace_section(n.bytes(), &Heading::new(2, "Plan"), "new\n", &s.hash).unwrap();
        assert_eq!(out, b"---\nStatus:\n  - Open\n---\n## Plan\nnew\n");
        assert!(matches!(
            n.field("Nope"),
            Err(VaultError::FieldNotFound { .. })
        ));
    }
}
