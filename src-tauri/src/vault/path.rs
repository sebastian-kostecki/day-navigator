//! The only way to name a file inside a vault.

use std::fmt;

use super::error::VaultError;

/// A validated, normalised vault-relative path made of whole segments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VaultPath {
    segments: Vec<String>,
}

impl VaultPath {
    /// Parses a vault-relative path. Accepts `/` or `\` as separators.
    pub fn parse(input: &str) -> Result<VaultPath, VaultError> {
        let invalid = |reason: &str| VaultError::InvalidPath {
            path: input.to_string(),
            reason: reason.to_string(),
        };
        if input.is_empty() {
            return Err(invalid("empty path"));
        }
        if input.starts_with('/') || input.starts_with('\\') {
            return Err(invalid("leading separator or UNC path"));
        }
        let mut chars = input.chars();
        if let (Some(first), Some(':')) = (chars.next(), chars.next())
            && first.is_ascii_alphabetic()
        {
            return Err(invalid("drive prefix"));
        }
        let mut segments = Vec::new();
        for seg in input.split(['/', '\\']) {
            validate_segment(seg, input)?;
            segments.push(seg.to_string());
        }
        Ok(VaultPath { segments })
    }

    /// Appends one validated segment.
    pub fn join(&self, name: &str) -> Result<VaultPath, VaultError> {
        validate_segment(name, name)?;
        let mut segments = self.segments.clone();
        segments.push(name.to_string());
        Ok(VaultPath { segments })
    }

    /// Builds a path from already-resolved segments without validation.
    /// Only for re-checking canonical remainders inside the guard.
    pub(crate) fn from_segments_unchecked(segments: Vec<String>) -> VaultPath {
        VaultPath { segments }
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    pub fn file_name(&self) -> Option<&str> {
        self.segments.last().map(String::as_str)
    }

    /// True if `prefix` is a whole-segment, case-insensitive prefix of `self`.
    /// Segments are compared in Unicode NFC: NTFS stores names as given, so a
    /// folder synced from macOS can be NFD (`e` + combining ogonek) while the
    /// settings file typed on Windows is NFC (`ę`).
    pub fn starts_with_ci(&self, prefix: &VaultPath) -> bool {
        self.segments.len() >= prefix.segments.len()
            && self
                .segments
                .iter()
                .zip(&prefix.segments)
                .all(|(a, b)| fold(a) == fold(b))
    }

    /// True for an `.md` extension, ignoring case.
    pub fn is_markdown(&self) -> bool {
        self.file_name()
            .and_then(|n| n.rsplit_once('.'))
            .is_some_and(|(stem, ext)| !stem.is_empty() && ext.eq_ignore_ascii_case("md"))
    }
}

impl fmt::Display for VaultPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.segments.join("/"))
    }
}

/// True if two segments name the same entry: NFC, case-insensitive.
pub(crate) fn segment_eq(a: &str, b: &str) -> bool {
    fold(a) == fold(b)
}

/// Comparison key for a segment: NFC, then lowercase.
fn fold(seg: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    seg.nfc().collect::<String>().to_lowercase()
}

fn validate_segment(seg: &str, whole: &str) -> Result<(), VaultError> {
    let invalid = |reason: &str| VaultError::InvalidPath {
        path: whole.to_string(),
        reason: reason.to_string(),
    };
    if seg.is_empty() {
        return Err(invalid("empty segment"));
    }
    if seg == "." || seg == ".." {
        return Err(invalid("dot segment"));
    }
    if seg.starts_with('.') {
        return Err(VaultError::Hidden {
            path: whole.to_string(),
        });
    }
    if seg
        .chars()
        .any(|c| c.is_control() || ":<>\"|?*/\\".contains(c))
    {
        return Err(invalid("forbidden character"));
    }
    if seg.ends_with('.') || seg.ends_with(' ') {
        return Err(invalid("segment ends with a dot or a space"));
    }
    if is_reserved_device_name(seg) {
        return Err(invalid("reserved device name"));
    }
    Ok(())
}

fn is_reserved_device_name(seg: &str) -> bool {
    let stem = seg.split('.').next().unwrap_or("").trim_end();
    let upper = stem.to_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(rest) = upper.strip_prefix(prefix) {
            let mut it = rest.chars();
            if let (Some(c), None) = (it.next(), it.next())
                && matches!(c, '1'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}')
            {
                return true;
            }
        }
    }
    false
}
