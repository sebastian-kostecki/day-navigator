//! Pure span finding on note bytes: frontmatter field blocks and ATX heading sections.
//!
//! Everything here works on `&[u8]` and never touches the disk. Spans are byte
//! ranges aligned to line starts, so an edit is `bytes[..start] ++ new ++ bytes[end..]`.

use sha2::{Digest, Sha256};

use super::error::VaultError;

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// An ATX heading: level 1..=6 and its text (without `#` markers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
}

impl Heading {
    pub fn new(level: u8, text: &str) -> Heading {
        Heading {
            level,
            text: text.to_string(),
        }
    }

    fn display(&self) -> String {
        format!("{} {}", "#".repeat(self.level as usize), self.text)
    }
}

/// Byte offsets `start..end`, aligned to line starts (or EOF).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn slice<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        &bytes[self.start..self.end]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Lf,
    CrLf,
}

impl Eol {
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::Lf => "\n",
            Eol::CrLf => "\r\n",
        }
    }
}

/// The first line ending in the file decides. LF if there is none.
pub fn detect_eol(bytes: &[u8]) -> Eol {
    match bytes.iter().position(|b| *b == b'\n') {
        Some(i) if i > 0 && bytes[i - 1] == b'\r' => Eol::CrLf,
        _ => Eol::Lf,
    }
}

/// SHA-256 (hex) of a span's bytes. Callers keep it to detect concurrent edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanHash(String);

impl SpanHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn span_hash(bytes: &[u8], span: Span) -> SpanHash {
    let digest = Sha256::digest(&bytes[span.start..span.end]);
    SpanHash(format!("{digest:x}"))
}

// ---------------------------------------------------------------------------
// Line scanning
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct Line {
    /// First byte of the line.
    start: usize,
    /// One past the line ending (or EOF).
    end: usize,
    /// End of the content, before `\n` / `\r\n`.
    text_end: usize,
}

impl Line {
    fn text<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        &bytes[self.start..self.text_end]
    }
}

fn lines(bytes: &[u8], from: usize, to: usize) -> Vec<Line> {
    let mut out = Vec::new();
    let mut pos = from;
    while pos < to {
        match bytes[pos..to].iter().position(|b| *b == b'\n') {
            Some(i) => {
                let nl = pos + i;
                let text_end = if nl > pos && bytes[nl - 1] == b'\r' {
                    nl - 1
                } else {
                    nl
                };
                out.push(Line {
                    start: pos,
                    end: nl + 1,
                    text_end,
                });
                pos = nl + 1;
            }
            None => {
                out.push(Line {
                    start: pos,
                    end: to,
                    text_end: to,
                });
                pos = to;
            }
        }
    }
    out
}

fn bom_len(bytes: &[u8]) -> usize {
    if bytes.starts_with(&BOM) {
        BOM.len()
    } else {
        0
    }
}

fn is_blank(text: &[u8]) -> bool {
    text.iter().all(|b| matches!(b, b' ' | b'\t' | b'\r'))
}

// ---------------------------------------------------------------------------
// Frontmatter
// ---------------------------------------------------------------------------

struct Frontmatter {
    /// First line after the opening `---`.
    body_start: usize,
    /// Start of the closing `---` / `...` line.
    body_end: usize,
    /// One past the closing line.
    end: usize,
}

/// Frontmatter exists only if the file (after the BOM) starts with a `---` line
/// and a later line is exactly `---` or `...`.
fn frontmatter(bytes: &[u8]) -> Option<Frontmatter> {
    let all = lines(bytes, bom_len(bytes), bytes.len());
    let first = all.first()?;
    if first.text(bytes) != b"---" || first.end == first.text_end {
        // Not `---`, or the opening line has no line ending at all.
        return None;
    }
    for line in &all[1..] {
        let t = line.text(bytes);
        if t == b"---" || t == b"..." {
            return Some(Frontmatter {
                body_start: first.end,
                body_end: line.start,
                end: line.end,
            });
        }
    }
    None
}

fn is_key_line(text: &[u8], key: &str) -> bool {
    let key = key.as_bytes();
    !key.is_empty()
        && text.starts_with(key)
        && text.get(key.len()) == Some(&b':')
        && matches!(text.get(key.len() + 1), None | Some(b' ') | Some(b'\t'))
}

fn is_continuation(text: &[u8]) -> bool {
    !is_blank(text)
        && (text.starts_with(b" ") || text.starts_with(b"\t") || text.starts_with(b"- "))
}

/// Span of a top-level frontmatter field: the `key:` line plus its indented or
/// `- ` continuation lines, including the final line ending.
pub fn find_field(bytes: &[u8], key: &str) -> Result<Span, VaultError> {
    let fm = frontmatter(bytes).ok_or(VaultError::NoFrontmatter)?;
    let ls = lines(bytes, fm.body_start, fm.body_end);
    let hits: Vec<usize> = ls
        .iter()
        .enumerate()
        .filter(|(_, l)| is_key_line(l.text(bytes), key))
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [] => Err(VaultError::FieldNotFound {
            key: key.to_string(),
        }),
        [i] => {
            let mut last = *i;
            while last + 1 < ls.len() && is_continuation(ls[last + 1].text(bytes)) {
                last += 1;
            }
            // An indented or `- ` line after blank or comment lines still belongs
            // to this field in YAML; a partial replace would leave it behind and
            // could break the Properties block. Refuse instead.
            let detached = ls[last + 1..]
                .iter()
                .map(|l| l.text(bytes))
                .find(|t| !is_blank(t) && !t.starts_with(b"#"))
                .is_some_and(is_continuation);
            if detached {
                return Err(VaultError::FieldLayoutUnsupported {
                    key: key.to_string(),
                });
            }
            Ok(Span {
                start: ls[*i].start,
                end: ls[last].end,
            })
        }
        _ => Err(VaultError::FieldAmbiguous {
            key: key.to_string(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

fn leading_spaces(text: &[u8]) -> usize {
    text.iter().take_while(|b| **b == b' ').count()
}

/// Opening code fence: 0-3 spaces, then 3+ backticks or tildes.
fn fence_open(text: &[u8]) -> Option<(u8, usize)> {
    let indent = leading_spaces(text);
    if indent > 3 {
        return None;
    }
    let rest = &text[indent..];
    let ch = *rest.first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let run = rest.iter().take_while(|b| **b == ch).count();
    if run < 3 {
        return None;
    }
    if ch == b'`' && rest[run..].contains(&b'`') {
        return None;
    }
    Some((ch, run))
}

fn fence_close(text: &[u8], ch: u8, len: usize) -> bool {
    let indent = leading_spaces(text);
    if indent > 3 {
        return false;
    }
    let rest = &text[indent..];
    let run = rest.iter().take_while(|b| **b == ch).count();
    run >= len && is_blank(&rest[run..])
}

/// CommonMark ATX heading: 0-3 spaces, 1-6 `#`, then space, tab or end of line.
fn parse_heading(text: &[u8]) -> Option<(u8, String)> {
    let indent = leading_spaces(text);
    if indent > 3 {
        return None;
    }
    let rest = &text[indent..];
    let hashes = rest.iter().take_while(|b| **b == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let after = &rest[hashes..];
    if !after.is_empty() && !matches!(after[0], b' ' | b'\t') {
        return None;
    }
    let mut content = trim_ws(after);
    // Optional closing sequence: trailing `#`s preceded by whitespace (or the whole content).
    let closing = content.iter().rev().take_while(|b| **b == b'#').count();
    if closing > 0 {
        let before = &content[..content.len() - closing];
        if before.is_empty() || matches!(before[before.len() - 1], b' ' | b'\t') {
            content = trim_ws(before);
        }
    }
    Some((hashes as u8, String::from_utf8_lossy(content).into_owned()))
}

fn trim_ws(mut s: &[u8]) -> &[u8] {
    while let Some((first, rest)) = s.split_first() {
        if matches!(first, b' ' | b'\t' | b'\r') {
            s = rest;
        } else {
            break;
        }
    }
    while let Some((last, rest)) = s.split_last() {
        if matches!(last, b' ' | b'\t' | b'\r') {
            s = rest;
        } else {
            break;
        }
    }
    s
}

/// All ATX headings outside frontmatter and fenced code, in document order.
fn headings(bytes: &[u8]) -> Vec<(Line, u8, String)> {
    let from = match frontmatter(bytes) {
        Some(fm) => fm.end,
        None => bom_len(bytes),
    };
    let mut out = Vec::new();
    let mut fence: Option<(u8, usize)> = None;
    for line in lines(bytes, from, bytes.len()) {
        let text = line.text(bytes);
        if let Some((ch, len)) = fence {
            if fence_close(text, ch, len) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(text) {
            fence = Some(open);
            continue;
        }
        if let Some((level, title)) = parse_heading(text) {
            out.push((line, level, title));
        }
    }
    out
}

/// Checks text about to be written into the body of `heading`'s section. A
/// heading at that level or higher (outside fences) would end the section
/// early, and an unclosed fence would hide every later heading; either would
/// change the structure of the rest of the note. Deeper subheadings are fine.
pub fn check_section_text(text: &[u8], heading: &Heading) -> Result<(), VaultError> {
    let mut fence: Option<(u8, usize)> = None;
    for line in lines(text, 0, text.len()) {
        let t = line.text(text);
        if let Some((ch, len)) = fence {
            if fence_close(t, ch, len) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(t) {
            fence = Some(open);
            continue;
        }
        if let Some((level, _)) = parse_heading(t) {
            if level <= heading.level {
                return Err(VaultError::InvalidValue {
                    reason: format!(
                        "text contains a level-{level} heading, which would end the section {}",
                        heading.display()
                    ),
                });
            }
        }
    }
    if fence.is_some() {
        return Err(VaultError::InvalidValue {
            reason: "text leaves a code fence open".to_string(),
        });
    }
    Ok(())
}

/// Span of a section body: from the line after the matching heading up to the
/// next heading of the same or higher level, or EOF.
pub fn find_section(bytes: &[u8], heading: &Heading) -> Result<Span, VaultError> {
    let all = headings(bytes);
    let wanted = heading.text.trim();
    let hits: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, (_, level, text))| *level == heading.level && text == wanted)
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [] => Err(VaultError::SectionNotFound {
            heading: heading.display(),
        }),
        [i] => {
            let end = all[*i + 1..]
                .iter()
                .find(|(_, level, _)| *level <= heading.level)
                .map(|(line, _, _)| line.start)
                .unwrap_or(bytes.len());
            Ok(Span {
                start: all[*i].0.end,
                end,
            })
        }
        many => Err(VaultError::SectionAmbiguous {
            heading: heading.display(),
            count: many.len(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Test support shared with edit.rs
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod testgen {
    use super::*;
    use proptest::prelude::*;

    #[derive(Debug, Clone)]
    pub enum FieldSpec {
        Scalar(String),
        List { indent: String, items: Vec<String> },
    }

    #[derive(Debug, Clone)]
    enum Frag {
        Heading {
            level: u8,
            text: &'static str,
            closed: bool,
        },
        Fence {
            tilde: bool,
            inner: Vec<&'static str>,
        },
        Text(&'static str),
        Blank,
    }

    /// A generated note plus the spans the generator knows it contains.
    #[derive(Debug, Clone)]
    pub struct GenNote {
        pub bytes: Vec<u8>,
        pub eol: Eol,
        pub target: Heading,
        pub body: Span,
        pub fields: Vec<(&'static str, FieldSpec, Span)>,
        pub has_frontmatter: bool,
    }

    pub const KEYS: [&str; 4] = ["Status", "Labels", "Priority", "Owner"];

    fn word() -> impl Strategy<Value = String> {
        "[A-Za-z0-9_]{1,6}"
    }

    pub fn scalar_value() -> impl Strategy<Value = String> {
        prop::collection::vec(word(), 1..3).prop_map(|w| w.join(" "))
    }

    fn field_spec() -> impl Strategy<Value = FieldSpec> {
        prop_oneof![
            scalar_value().prop_map(FieldSpec::Scalar),
            (
                prop::sample::select(vec!["", "  ", "    "]),
                prop::collection::vec(scalar_value(), 1..4)
            )
                .prop_map(|(indent, items)| FieldSpec::List {
                    indent: indent.to_string(),
                    items
                }),
        ]
    }

    fn frag() -> impl Strategy<Value = Frag> {
        prop_oneof![
            (
                1u8..=6,
                prop::sample::select(vec!["A", "B", "C"]),
                any::<bool>()
            )
                .prop_map(|(level, text, closed)| Frag::Heading {
                    level,
                    text,
                    closed
                }),
            (
                any::<bool>(),
                prop::collection::vec(
                    prop::sample::select(vec![
                        "## Target",
                        "# Target",
                        "### A",
                        "~~~",
                        "```",
                        "text",
                        ""
                    ]),
                    0..4
                )
            )
                .prop_map(|(tilde, inner)| Frag::Fence { tilde, inner }),
            prop::sample::select(vec![
                "plain text",
                "#tag",
                "#tag more",
                "##Target",
                "    ## Target",
                "---",
                "Status: from body",
                "- list item",
                "####### seven",
            ])
            .prop_map(Frag::Text),
            Just(Frag::Blank),
        ]
    }

    fn eol_strategy() -> impl Strategy<Value = Eol> {
        prop_oneof![Just(Eol::Lf), Just(Eol::CrLf)]
    }

    pub fn gen_note() -> impl Strategy<Value = GenNote> {
        (
            any::<bool>(),                                                              // BOM
            prop::option::of(prop::collection::vec(prop::option::of(field_spec()), 4)), // frontmatter
            prop::collection::vec(frag(), 0..12),
            any::<usize>(),             // target position
            1u8..=6,                    // target level
            (any::<bool>(), 0usize..4), // target closed / indent
            eol_strategy(),
            any::<bool>(), // trailing EOL
        )
            .prop_map(
                |(bom, fm, frags, pos, level, (closed, indent), eol, trailing)| {
                    render(bom, fm, frags, pos, level, closed, indent, eol, trailing)
                },
            )
    }

    #[allow(clippy::too_many_arguments)]
    fn render(
        bom: bool,
        fm: Option<Vec<Option<FieldSpec>>>,
        frags: Vec<Frag>,
        pos: usize,
        level: u8,
        closed: bool,
        indent: usize,
        eol: Eol,
        trailing: bool,
    ) -> GenNote {
        // Lines are (text, kind): kind marks the target and level-bearing headings.
        enum Kind {
            Other,
            Target,
            Heading(u8),
        }
        let mut lines_out: Vec<(String, Kind)> = Vec::new();
        let pos = pos % (frags.len() + 1);
        let target_text = format!(
            "{}{} Target{}",
            " ".repeat(indent),
            "#".repeat(level as usize),
            if closed { " ##" } else { "" }
        );
        for (i, f) in frags.iter().enumerate() {
            if i == pos {
                lines_out.push((target_text.clone(), Kind::Target));
            }
            match f {
                Frag::Heading {
                    level,
                    text,
                    closed,
                } => lines_out.push((
                    format!(
                        "{} {}{}",
                        "#".repeat(*level as usize),
                        text,
                        if *closed { " #" } else { "" }
                    ),
                    Kind::Heading(*level),
                )),
                Frag::Fence { tilde, inner } => {
                    let fence = if *tilde { "~~~~" } else { "````" };
                    lines_out.push((fence.to_string(), Kind::Other));
                    for l in inner {
                        lines_out.push((l.to_string(), Kind::Other));
                    }
                    lines_out.push((fence.to_string(), Kind::Other));
                }
                Frag::Text(t) => lines_out.push((t.to_string(), Kind::Other)),
                Frag::Blank => lines_out.push((String::new(), Kind::Other)),
            }
        }
        if pos == frags.len() {
            lines_out.push((target_text, Kind::Target));
        }
        // A leading `---` text line would turn into frontmatter; keep that out.
        if fm.is_none() && lines_out.first().is_some_and(|(t, _)| t == "---") {
            lines_out.insert(0, ("intro".to_string(), Kind::Other));
        }

        let eol_s = eol.as_str();
        let mut bytes: Vec<u8> = Vec::new();
        if bom {
            bytes.extend_from_slice(&BOM);
        }
        let mut fields = Vec::new();
        let has_frontmatter = fm.is_some();
        if let Some(specs) = fm {
            bytes.extend_from_slice(format!("---{eol_s}").as_bytes());
            for (key, spec) in KEYS.iter().zip(specs) {
                let Some(spec) = spec else { continue };
                let start = bytes.len();
                match &spec {
                    FieldSpec::Scalar(v) => {
                        bytes.extend_from_slice(format!("{key}: {v}{eol_s}").as_bytes())
                    }
                    FieldSpec::List { indent, items } => {
                        bytes.extend_from_slice(format!("{key}:{eol_s}").as_bytes());
                        for it in items {
                            bytes.extend_from_slice(format!("{indent}- {it}{eol_s}").as_bytes());
                        }
                    }
                }
                fields.push((
                    *key,
                    spec,
                    Span {
                        start,
                        end: bytes.len(),
                    },
                ));
            }
            bytes.extend_from_slice(format!("---{eol_s}").as_bytes());
        }

        let count = lines_out.len();
        let mut body_start = 0;
        let mut body_end = None;
        let mut seen_target = false;
        for (i, (text, kind)) in lines_out.iter().enumerate() {
            let line_start = bytes.len();
            match kind {
                Kind::Heading(l) if seen_target && *l <= level && body_end.is_none() => {
                    body_end = Some(line_start)
                }
                Kind::Target => seen_target = true,
                _ => {}
            }
            bytes.extend_from_slice(text.as_bytes());
            if i + 1 < count || trailing {
                bytes.extend_from_slice(eol_s.as_bytes());
            }
            if matches!(kind, Kind::Target) {
                body_start = bytes.len();
            }
        }
        let body = Span {
            start: body_start,
            end: body_end.unwrap_or(bytes.len()),
        };
        let eol = detect_eol(&bytes);
        GenNote {
            bytes,
            eol,
            target: Heading::new(level, "Target"),
            body,
            fields,
            has_frontmatter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(bytes: &[u8], span: Span) -> String {
        String::from_utf8(span.slice(bytes).to_vec()).unwrap()
    }

    fn plan() -> Heading {
        Heading::new(2, "Plan")
    }

    #[test]
    fn section_runs_through_deeper_headings_to_next_same_level() {
        let doc = b"## Plan\na\n### Notatki\nb\n## Log\nc\n";
        let span = find_section(doc, &plan()).unwrap();
        assert_eq!(text_of(doc, span), "a\n### Notatki\nb\n");
    }

    #[test]
    fn section_ends_at_higher_level_heading() {
        let doc = b"## Plan\na\n# Top\nb\n";
        assert_eq!(text_of(doc, find_section(doc, &plan()).unwrap()), "a\n");
    }

    #[test]
    fn section_to_eof_when_no_following_heading() {
        let doc = b"## Plan\na\nb";
        let span = find_section(doc, &plan()).unwrap();
        assert_eq!(text_of(doc, span), "a\nb");
        assert_eq!(span.end, doc.len());
    }

    #[test]
    fn heading_inside_backtick_fence_is_ignored() {
        let doc = b"## Plan\n```\n## Log\n```\nx\n## Log\ny\n";
        let span = find_section(doc, &plan()).unwrap();
        assert_eq!(text_of(doc, span), "```\n## Log\n```\nx\n");
        // The fenced `## Log` must not count: only one real `## Log`.
        assert!(find_section(doc, &Heading::new(2, "Log")).is_ok());
    }

    #[test]
    fn tilde_fence_needs_matching_char_and_length() {
        // A shorter ~~ run and a backtick run do not close a ~~~~ fence.
        let doc = b"## Plan\n~~~~\n## Log\n~~~\n```\n~~~~\nz\n## Log\n";
        let span = find_section(doc, &plan()).unwrap();
        assert_eq!(text_of(doc, span), "~~~~\n## Log\n~~~\n```\n~~~~\nz\n");
    }

    #[test]
    fn line_start_tag_does_not_end_section() {
        let doc = b"## Plan\n#tag one\nmore\n## Log\n";
        let span = find_section(doc, &plan()).unwrap();
        assert_eq!(text_of(doc, span), "#tag one\nmore\n");
    }

    #[test]
    fn indented_four_spaces_and_seven_hashes_are_not_headings() {
        let doc = b"## Plan\n    ## Log\n####### x\n## Log\n";
        let span = find_section(doc, &plan()).unwrap();
        assert_eq!(text_of(doc, span), "    ## Log\n####### x\n");
        let doc = b"   ## Plan\nbody\n";
        assert!(
            find_section(doc, &plan()).is_ok(),
            "3 spaces of indent is a heading"
        );
    }

    #[test]
    fn closing_hashes_and_trailing_space_are_stripped_from_heading_text() {
        let doc = b"## Plan ##  \nbody\n";
        assert_eq!(text_of(doc, find_section(doc, &plan()).unwrap()), "body\n");
        // `#` not preceded by whitespace belongs to the text.
        let doc = b"## C#\nbody\n";
        assert!(find_section(doc, &Heading::new(2, "C#")).is_ok());
    }

    #[test]
    fn duplicate_heading_is_ambiguous() {
        let doc = b"## Plan\na\n## Plan\nb\n";
        match find_section(doc, &plan()) {
            Err(VaultError::SectionAmbiguous { count, .. }) => assert_eq!(count, 2),
            other => panic!("expected SectionAmbiguous, got {other:?}"),
        }
    }

    #[test]
    fn missing_heading_or_wrong_level_is_not_found() {
        let doc = b"## Plan\na\n";
        assert!(matches!(
            find_section(doc, &Heading::new(3, "Plan")),
            Err(VaultError::SectionNotFound { .. })
        ));
        assert!(matches!(
            find_section(doc, &Heading::new(2, "Pla")),
            Err(VaultError::SectionNotFound { .. })
        ));
    }

    #[test]
    fn heading_on_last_line_without_eol_has_empty_body_at_eof() {
        let doc = b"x\n## Plan";
        let span = find_section(doc, &plan()).unwrap();
        assert_eq!(
            span,
            Span {
                start: doc.len(),
                end: doc.len()
            }
        );
        assert!(span.is_empty());
    }

    #[test]
    fn heading_in_frontmatter_is_ignored() {
        let doc = b"---\n## Plan\n---\n## Plan\nbody\n";
        let span = find_section(doc, &plan()).unwrap();
        assert_eq!(text_of(doc, span), "body\n");
    }

    #[test]
    fn crlf_sections_end_on_line_boundaries() {
        let doc = b"## Plan\r\na\r\n## Log\r\n";
        let span = find_section(doc, &plan()).unwrap();
        assert_eq!(text_of(doc, span), "a\r\n");
        assert_eq!(detect_eol(doc), Eol::CrLf);
    }

    #[test]
    fn bom_is_skipped_before_frontmatter_and_headings() {
        let mut doc = BOM.to_vec();
        doc.extend_from_slice(b"---\nStatus: Open\n---\n## Plan\nbody\n");
        let f = find_field(&doc, "Status").unwrap();
        assert_eq!(text_of(&doc, f), "Status: Open\n");
        assert_eq!(f.start, 3 + 4);
        let s = find_section(&doc, &plan()).unwrap();
        assert_eq!(text_of(&doc, s), "body\n");
        // A BOM then a heading on the first line is still a heading.
        let mut doc = BOM.to_vec();
        doc.extend_from_slice(b"## Plan\nx\n");
        assert_eq!(find_section(&doc, &plan()).unwrap().start, 3 + 8);
    }

    #[test]
    fn fixture_daily_note_plan_includes_notatki_and_ignores_fenced_heading() {
        let doc = include_bytes!("../../tests/fixtures/vault/Dziennik/2026/2026-01-05.md");
        let span = find_section(doc, &plan()).unwrap();
        let body = text_of(doc, span);
        assert!(body.contains("### Notatki"));
        assert!(body.contains("## Not a heading"));
        assert!(!body.contains("## Log"));
        assert!(matches!(
            find_section(doc, &Heading::new(2, "Not a heading")),
            Err(VaultError::SectionNotFound { .. })
        ));
        assert!(find_section(doc, &Heading::new(3, "Notatki")).is_ok());
    }

    #[test]
    fn detect_eol_first_line_ending_wins() {
        assert_eq!(detect_eol(b"a\r\nb\n"), Eol::CrLf);
        assert_eq!(detect_eol(b"a\nb\r\n"), Eol::Lf);
        assert_eq!(detect_eol(b"no newline"), Eol::Lf);
        assert_eq!(detect_eol(b""), Eol::Lf);
    }

    #[test]
    fn list_field_is_one_block() {
        let doc = b"---\nStatus:\n  - Open\n  - In review\nLabels: x\n---\nbody\n";
        let span = find_field(doc, "Status").unwrap();
        assert_eq!(text_of(doc, span), "Status:\n  - Open\n  - In review\n");
        let span = find_field(doc, "Labels").unwrap();
        assert_eq!(text_of(doc, span), "Labels: x\n");
    }

    #[test]
    fn unindented_dash_items_belong_to_the_block() {
        let doc = b"---\nStatus:\n- a\n- b\nNext: 1\n---\n";
        assert_eq!(
            text_of(doc, find_field(doc, "Status").unwrap()),
            "Status:\n- a\n- b\n"
        );
    }

    #[test]
    fn field_block_stops_at_comment_blank_line_and_next_key() {
        let doc = b"---\nA:\n  - 1\n# note\nB: x\n\nC: y\n---\n";
        assert_eq!(text_of(doc, find_field(doc, "A").unwrap()), "A:\n  - 1\n");
        assert_eq!(text_of(doc, find_field(doc, "B").unwrap()), "B: x\n");
        let doc = b"---\nA: 1\n\nB: 2\n---\n";
        assert_eq!(text_of(doc, find_field(doc, "A").unwrap()), "A: 1\n");
    }

    #[test]
    fn list_items_after_blank_or_comment_lines_are_refused_not_split() {
        // YAML still counts `- 2` as part of A: a partial replace would leave it behind.
        for doc in [
            &b"---\nA:\n  - 1\n# note\n  - 2\nB: x\n---\n"[..],
            &b"---\nA:\n  - 1\n\n  - 2\n---\n"[..],
            &b"---\nA: x\n\n# c\n\n- lost\n---\n"[..],
        ] {
            assert!(
                matches!(
                    find_field(doc, "A"),
                    Err(VaultError::FieldLayoutUnsupported { .. })
                ),
                "{}",
                String::from_utf8_lossy(doc)
            );
        }
        // Other fields of the same note stay editable.
        let doc = b"---\nA:\n  - 1\n\n  - 2\nB: x\n---\n";
        assert_eq!(text_of(doc, find_field(doc, "B").unwrap()), "B: x\n");
    }

    #[test]
    fn field_key_is_case_sensitive_and_needs_a_colon_boundary() {
        let doc = b"---\nstatus: a\nStatusX: b\nStatus:c\n---\n";
        assert!(matches!(
            find_field(doc, "Status"),
            Err(VaultError::FieldNotFound { .. })
        ));
        assert!(find_field(doc, "status").is_ok());
    }

    #[test]
    fn duplicate_field_is_ambiguous() {
        let doc = b"---\nA: 1\nA: 2\n---\n";
        assert!(matches!(
            find_field(doc, "A"),
            Err(VaultError::FieldAmbiguous { .. })
        ));
    }

    #[test]
    fn no_frontmatter_cases() {
        for doc in [
            &b"A: 1\n"[..],
            b"\n---\nA: 1\n---\n",    // not at the start
            b"---\nA: 1\n",           // never closed
            b"# T\n---\nA: 1\n---\n", // later `---` is not frontmatter
        ] {
            assert!(
                matches!(find_field(doc, "A"), Err(VaultError::NoFrontmatter)),
                "{:?}",
                String::from_utf8_lossy(doc)
            );
        }
    }

    #[test]
    fn frontmatter_may_close_with_three_dots_and_crlf() {
        let doc = b"---\r\nA: 1\r\n...\r\nA: later\r\n";
        let span = find_field(doc, "A").unwrap();
        assert_eq!(text_of(doc, span), "A: 1\r\n");
    }

    #[test]
    fn span_hash_tracks_content() {
        let a = b"## Plan\none\n";
        let b = b"## Plan\ntwo\n";
        let sa = find_section(a, &plan()).unwrap();
        let sb = find_section(b, &plan()).unwrap();
        assert_eq!(span_hash(a, sa), span_hash(a, sa));
        assert_ne!(span_hash(a, sa), span_hash(b, sb));
        // Known SHA-256 of the empty string.
        assert_eq!(
            span_hash(b"", Span { start: 0, end: 0 }).as_str(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}

#[cfg(test)]
mod proptests {
    use super::testgen::*;
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Generated notes: the finders return exactly the spans the generator built.
        #[test]
        fn spans_match_generator_model(note in gen_note()) {
            let s = find_section(&note.bytes, &note.target).unwrap();
            prop_assert_eq!(s, note.body);
            for (key, _spec, span) in &note.fields {
                prop_assert_eq!(find_field(&note.bytes, key).unwrap(), *span);
            }
            if !note.has_frontmatter {
                prop_assert!(matches!(
                    find_field(&note.bytes, "Status"),
                    Err(VaultError::NoFrontmatter)
                ));
            }
        }

        /// Arbitrary input never panics; any span found is line-aligned and in range.
        #[test]
        fn finders_never_panic_and_spans_are_line_aligned(
            raw in prop::collection::vec(any::<u8>(), 0..300),
            tricky in prop::collection::vec(
                prop::sample::select(b"-#`~ \n\r.:aS".to_vec()), 0..300),
        ) {
            for bytes in [raw, tricky] {
                let mut spans = Vec::new();
                spans.extend(find_field(&bytes, "S").ok());
                spans.extend(find_field(&bytes, "a").ok());
                for level in 1..=6u8 {
                    spans.extend(find_section(&bytes, &Heading::new(level, "a")).ok());
                    spans.extend(find_section(&bytes, &Heading::new(level, "")).ok());
                }
                let _ = detect_eol(&bytes);
                for span in spans {
                    prop_assert!(span.start <= span.end && span.end <= bytes.len());
                    prop_assert!(span.start == 0 || span.start == bytes.len() || bytes[span.start - 1] == b'\n');
                    prop_assert!(span.end == bytes.len() || bytes[span.end - 1] == b'\n');
                    let _ = span_hash(&bytes, span);
                }
            }
        }
    }
}
