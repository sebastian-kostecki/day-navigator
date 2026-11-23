//! Pure edits: bytes in, bytes out. Each primitive builds the complete new file
//! as `original[..start] ++ replacement ++ original[end..]`, with inserted text
//! normalised to the file's own line ending.
//!
//! `Conflict` errors from this layer carry an empty `path`: the pure layer does
//! not know the note's vault path, so the caller fills it in.

use super::error::VaultError;
use super::markdown::{
    Eol, Heading, Span, SpanHash, check_section_text, detect_eol, find_field, find_section,
    span_hash,
};

/// New value for a frontmatter field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    Scalar(String),
    List(Vec<String>),
    /// `key:` with nothing after it (YAML null); clears a scalar field.
    Empty,
}

fn conflict() -> VaultError {
    VaultError::Conflict {
        path: String::new(),
    }
}

fn invalid(reason: &str) -> VaultError {
    VaultError::InvalidValue {
        reason: reason.to_string(),
    }
}

/// Values are written verbatim, so anything that would need YAML quoting is refused.
fn check_plain(value: &str) -> Result<(), VaultError> {
    if value.contains(['\r', '\n']) {
        return Err(invalid("value contains a line break"));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid("value contains a control character"));
    }
    if value.is_empty() {
        return Err(invalid("value is empty"));
    }
    if value.starts_with(char::is_whitespace) || value.ends_with(char::is_whitespace) {
        return Err(invalid("value has leading or trailing whitespace"));
    }
    if value.contains(": ") || value.ends_with(':') {
        return Err(invalid("value contains ': ' and would need quoting"));
    }
    if value.contains(" #") || value.starts_with('#') {
        return Err(invalid("value contains ' #' and would need quoting"));
    }
    if value.starts_with(|c| "[]{}&*!|>'\"%@`,".contains(c)) {
        return Err(invalid(
            "value starts with a YAML indicator and would need quoting",
        ));
    }
    if value.starts_with("- ") || value.starts_with("? ") || value == "-" || value == "?" {
        return Err(invalid(
            "value starts with a YAML indicator and would need quoting",
        ));
    }
    Ok(())
}

/// Indent of the first `- ` item in an existing block, or two spaces.
fn block_indent(block: &[u8]) -> String {
    for line in block.split(|b| *b == b'\n').skip(1) {
        let indent = line
            .iter()
            .take_while(|b| matches!(b, b' ' | b'\t'))
            .count();
        if line[indent..].starts_with(b"-") {
            return String::from_utf8_lossy(&line[..indent]).into_owned();
        }
    }
    "  ".to_string()
}

fn render_field(
    key: &str,
    value: &FieldValue,
    eol: Eol,
    indent: &str,
) -> Result<String, VaultError> {
    let nl = eol.as_str();
    match value {
        FieldValue::Scalar(v) => {
            check_plain(v)?;
            Ok(format!("{key}: {v}{nl}"))
        }
        FieldValue::Empty => Ok(format!("{key}:{nl}")),
        FieldValue::List(items) if items.is_empty() => Ok(format!("{key}: []{nl}")),
        FieldValue::List(items) => {
            let mut out = format!("{key}:{nl}");
            for item in items {
                check_plain(item)?;
                out.push_str(&format!("{indent}- {item}{nl}"));
            }
            Ok(out)
        }
    }
}

fn splice(bytes: &[u8], span: Span, replacement: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() - (span.end - span.start) + replacement.len());
    out.extend_from_slice(&bytes[..span.start]);
    out.extend_from_slice(replacement);
    out.extend_from_slice(&bytes[span.end..]);
    out
}

/// Converts every line break in `text` to `eol`.
fn normalize_eol(text: &str, eol: Eol) -> String {
    let lf = text.replace("\r\n", "\n").replace('\r', "\n");
    match eol {
        Eol::Lf => lf,
        Eol::CrLf => lf.replace('\n', "\r\n"),
    }
}

/// True when `pos` is not the start of a line, i.e. the previous line has no ending.
fn mid_line(bytes: &[u8], pos: usize) -> bool {
    pos > 0 && bytes[pos - 1] != b'\n'
}

pub fn replace_field(
    bytes: &[u8],
    key: &str,
    value: &FieldValue,
    expected: &SpanHash,
) -> Result<Vec<u8>, VaultError> {
    let span = find_field(bytes, key)?;
    if span_hash(bytes, span) != *expected {
        return Err(conflict());
    }
    let indent = block_indent(span.slice(bytes));
    let text = render_field(key, value, detect_eol(bytes), &indent)?;
    Ok(splice(bytes, span, text.as_bytes()))
}

/// Replaces several fields at once. Every hash is checked against the original
/// bytes first; only then are the fields spliced one after another, each span
/// found again in the progressively edited bytes.
pub fn replace_fields(
    bytes: &[u8],
    edits: &[(&str, FieldValue, SpanHash)],
) -> Result<Vec<u8>, VaultError> {
    for (i, (key, _, _)) in edits.iter().enumerate() {
        if edits[..i].iter().any(|(k, _, _)| k == key) {
            return Err(invalid("duplicate field in edit list"));
        }
    }
    for (key, _, expected) in edits {
        let span = find_field(bytes, key)?;
        if span_hash(bytes, span) != *expected {
            return Err(conflict());
        }
    }
    let mut out = bytes.to_vec();
    for (key, value, _) in edits {
        let span = find_field(&out, key)?;
        let indent = block_indent(span.slice(&out));
        let text = render_field(key, value, detect_eol(&out), &indent)?;
        out = splice(&out, span, text.as_bytes());
    }
    Ok(out)
}

pub fn replace_section(
    bytes: &[u8],
    heading: &Heading,
    new_body: &str,
    expected: &SpanHash,
) -> Result<Vec<u8>, VaultError> {
    let span = find_section(bytes, heading)?;
    if span_hash(bytes, span) != *expected {
        return Err(conflict());
    }
    let eol = detect_eol(bytes);
    let mut text = normalize_eol(new_body, eol);
    check_section_text(text.as_bytes(), heading)?;
    if !text.is_empty() {
        if mid_line(bytes, span.start) {
            // The heading is the last line and has no ending yet.
            text.insert_str(0, eol.as_str());
        }
        if span.end < bytes.len() && !text.ends_with(eol.as_str()) {
            text.push_str(eol.as_str());
        }
    }
    Ok(splice(bytes, span, text.as_bytes()))
}

pub fn append_to_section(
    bytes: &[u8],
    heading: &Heading,
    lines: &str,
) -> Result<Vec<u8>, VaultError> {
    if lines.is_empty() {
        return Err(invalid("nothing to append"));
    }
    let span = find_section(bytes, heading)?;
    let eol = detect_eol(bytes);
    check_section_text(normalize_eol(lines, eol).as_bytes(), heading)?;
    // Insert after the last non-blank body line; a blank body inserts right after the heading.
    let body = span.slice(bytes);
    let mut pos = span.start;
    let mut offset = 0;
    for chunk in body.split_inclusive(|b| *b == b'\n') {
        offset += chunk.len();
        if !chunk
            .iter()
            .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        {
            pos = span.start + offset;
        }
    }
    let mut text = String::new();
    if mid_line(bytes, pos) {
        text.push_str(eol.as_str());
    }
    text.push_str(&normalize_eol(lines, eol));
    if !text.ends_with(eol.as_str()) {
        text.push_str(eol.as_str());
    }
    Ok(splice(
        bytes,
        Span {
            start: pos,
            end: pos,
        },
        text.as_bytes(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

    fn field_hash(bytes: &[u8], key: &str) -> SpanHash {
        span_hash(bytes, find_field(bytes, key).unwrap())
    }

    fn section_hash(bytes: &[u8], h: &Heading) -> SpanHash {
        span_hash(bytes, find_section(bytes, h).unwrap())
    }

    fn s(bytes: Vec<u8>) -> String {
        String::from_utf8(bytes).unwrap()
    }

    fn scalar(v: &str) -> FieldValue {
        FieldValue::Scalar(v.to_string())
    }

    fn list(items: &[&str]) -> FieldValue {
        FieldValue::List(items.iter().map(|i| i.to_string()).collect())
    }

    #[test]
    fn list_valued_status_is_replaced_as_one_block() {
        let doc = b"---\nStatus:\n  - Open\n  - In review\nLabels: x\n---\nbody\n";
        let out =
            replace_field(doc, "Status", &list(&["Done"]), &field_hash(doc, "Status")).unwrap();
        assert_eq!(s(out), "---\nStatus:\n  - Done\nLabels: x\n---\nbody\n");
    }

    #[test]
    fn list_replacement_copies_existing_indent_else_two_spaces() {
        let doc = b"---\nA:\n    - x\nB: 1\n---\n";
        let out = replace_field(doc, "A", &list(&["p", "q"]), &field_hash(doc, "A")).unwrap();
        assert_eq!(s(out), "---\nA:\n    - p\n    - q\nB: 1\n---\n");
        let doc = b"---\nA: x\n---\n";
        let out = replace_field(doc, "A", &list(&["p"]), &field_hash(doc, "A")).unwrap();
        assert_eq!(s(out), "---\nA:\n  - p\n---\n");
        let doc = b"---\nA:\n- x\n---\n";
        let out = replace_field(doc, "A", &list(&["p"]), &field_hash(doc, "A")).unwrap();
        assert_eq!(s(out), "---\nA:\n- p\n---\n");
    }

    #[test]
    fn scalar_replaces_a_list_block_and_back() {
        let doc = b"---\nA:\n  - x\n  - y\nB: 1\n---\n";
        let out = replace_field(doc, "A", &scalar("z"), &field_hash(doc, "A")).unwrap();
        assert_eq!(s(out), "---\nA: z\nB: 1\n---\n");
    }

    #[test]
    fn empty_list_renders_as_empty_flow_list() {
        let doc = b"---\nA:\n  - x\nB: 1\n---\n";
        let out = replace_field(doc, "A", &list(&[]), &field_hash(doc, "A")).unwrap();
        assert_eq!(s(out), "---\nA: []\nB: 1\n---\n");
    }

    #[test]
    fn crlf_file_gets_crlf_in_inserted_field_text() {
        let doc = b"---\r\nA: x\r\nB: 1\r\n---\r\nbody\r\n";
        let out = replace_field(doc, "A", &list(&["p", "q"]), &field_hash(doc, "A")).unwrap();
        assert_eq!(
            s(out),
            "---\r\nA:\r\n  - p\r\n  - q\r\nB: 1\r\n---\r\nbody\r\n"
        );
    }

    #[test]
    fn crlf_file_gets_crlf_in_inserted_section_text() {
        let doc = b"## Plan\r\nold\r\n## Log\r\n";
        let h = Heading::new(2, "Plan");
        let out = replace_section(doc, &h, "a\nb", &section_hash(doc, &h)).unwrap();
        assert_eq!(s(out), "## Plan\r\na\r\nb\r\n## Log\r\n");
        let out = append_to_section(doc, &h, "x\ny").unwrap();
        assert_eq!(s(out), "## Plan\r\nold\r\nx\r\ny\r\n## Log\r\n");
    }

    #[test]
    fn bom_is_preserved_at_offset_zero() {
        let mut doc = BOM.to_vec();
        doc.extend_from_slice(b"---\nA: x\n---\n## Plan\nold\n");
        let out = replace_field(&doc, "A", &scalar("y"), &field_hash(&doc, "A")).unwrap();
        assert_eq!(&out[..3], &BOM);
        assert_eq!(&out[3..], b"---\nA: y\n---\n## Plan\nold\n");
        let h = Heading::new(2, "Plan");
        let out = replace_section(&doc, &h, "new\n", &section_hash(&doc, &h)).unwrap();
        assert_eq!(&out[..3], &BOM);
        assert!(out.ends_with(b"## Plan\nnew\n"));
        let out = append_to_section(&doc, &h, "more").unwrap();
        assert_eq!(&out[..3], &BOM);
        assert!(out.ends_with(b"## Plan\nold\nmore\n"));
    }

    #[test]
    fn empty_value_renders_key_and_colon_with_the_files_eol() {
        let doc = b"---\nDone: 2026-01-10\nB: 1\n---\n";
        let out = replace_field(doc, "Done", &FieldValue::Empty, &field_hash(doc, "Done")).unwrap();
        assert_eq!(s(out), "---\nDone:\nB: 1\n---\n");
        let doc = b"---\r\nDone: 2026-01-10\r\nB: 1\r\n---\r\n";
        let out = replace_field(doc, "Done", &FieldValue::Empty, &field_hash(doc, "Done")).unwrap();
        assert_eq!(s(out), "---\r\nDone:\r\nB: 1\r\n---\r\n");
        // An empty string is still refused.
        assert!(matches!(
            replace_field(doc, "Done", &scalar(""), &field_hash(doc, "Done")),
            Err(VaultError::InvalidValue { .. })
        ));
    }

    #[test]
    fn replace_fields_changes_status_and_done_in_one_pass() {
        let doc = b"---\nStatus: Todo\nDue: x\nDone:\n---\nbody\n";
        let edits = [
            ("Status", scalar("Done"), field_hash(doc, "Status")),
            ("Done", scalar("2026-02-01"), field_hash(doc, "Done")),
        ];
        let out = replace_fields(doc, &edits).unwrap();
        assert_eq!(
            s(out),
            "---\nStatus: Done\nDue: x\nDone: 2026-02-01\n---\nbody\n"
        );
        let doc = b"---\r\nStatus: Done\r\nDone: 2026-01-10\r\n---\r\n";
        let edits = [
            ("Done", FieldValue::Empty, field_hash(doc, "Done")),
            ("Status", scalar("Todo"), field_hash(doc, "Status")),
        ];
        assert_eq!(
            s(replace_fields(doc, &edits).unwrap()),
            "---\r\nStatus: Todo\r\nDone:\r\n---\r\n"
        );
        assert_eq!(replace_fields(doc, &[]).unwrap(), doc);
    }

    #[test]
    fn replace_fields_conflicts_when_any_hash_is_stale_and_returns_no_bytes() {
        let doc = b"---\nStatus: Todo\nDone:\n---\n";
        let stale = field_hash(b"---\nDone: other\n---\n", "Done");
        let edits = [
            ("Status", scalar("Done"), field_hash(doc, "Status")),
            ("Done", scalar("2026-02-01"), stale),
        ];
        assert!(matches!(
            replace_fields(doc, &edits),
            Err(VaultError::Conflict { .. })
        ));
    }

    #[test]
    fn replace_fields_refuses_duplicate_keys_and_missing_fields() {
        let doc = b"---\nStatus: Todo\n---\n";
        let h = field_hash(doc, "Status");
        let dup = [
            ("Status", scalar("A"), h.clone()),
            ("Status", scalar("B"), h.clone()),
        ];
        assert!(matches!(
            replace_fields(doc, &dup),
            Err(VaultError::InvalidValue { .. })
        ));
        let missing = [("Nope", scalar("A"), h)];
        assert!(matches!(
            replace_fields(doc, &missing),
            Err(VaultError::FieldNotFound { .. })
        ));
    }

    #[test]
    fn heading_on_last_line_without_eol_gets_one_eol_first() {
        let doc = b"x\n## Plan";
        let h = Heading::new(2, "Plan");
        let out = append_to_section(doc, &h, "item").unwrap();
        assert_eq!(s(out), "x\n## Plan\nitem\n");
        let out = replace_section(doc, &h, "item", &section_hash(doc, &h)).unwrap();
        assert_eq!(s(out), "x\n## Plan\nitem");
        let crlf = b"x\r\n## Plan";
        let out = append_to_section(crlf, &h, "item").unwrap();
        assert_eq!(s(out), "x\r\n## Plan\r\nitem\r\n");
    }

    #[test]
    fn replace_section_adds_trailing_eol_only_when_not_at_eof() {
        let h = Heading::new(2, "Plan");
        let doc = b"## Plan\nold\n## Log\n";
        let out = replace_section(doc, &h, "new", &section_hash(doc, &h)).unwrap();
        assert_eq!(s(out), "## Plan\nnew\n## Log\n");
        let doc = b"## Plan\nold\n";
        let out = replace_section(doc, &h, "new", &section_hash(doc, &h)).unwrap();
        assert_eq!(s(out), "## Plan\nnew");
    }

    #[test]
    fn append_goes_after_last_nonblank_line_keeping_blank_lines_before_next_heading() {
        let h = Heading::new(2, "Plan");
        let doc = b"## Plan\n- a\n- b\n\n\n## Log\n";
        let out = append_to_section(doc, &h, "- c").unwrap();
        assert_eq!(s(out), "## Plan\n- a\n- b\n- c\n\n\n## Log\n");
    }

    #[test]
    fn append_to_blank_body_goes_right_after_heading() {
        let h = Heading::new(2, "Plan");
        let doc = b"## Plan\n\n\n## Log\n";
        let out = append_to_section(doc, &h, "- c\n").unwrap();
        assert_eq!(s(out), "## Plan\n- c\n\n\n## Log\n");
    }

    #[test]
    fn append_at_eof_without_final_eol_adds_one_before_the_new_line() {
        let h = Heading::new(2, "Plan");
        let doc = b"## Plan\n- a";
        let out = append_to_section(doc, &h, "- b").unwrap();
        assert_eq!(s(out), "## Plan\n- a\n- b\n");
    }

    #[test]
    fn append_rejects_empty_input() {
        let doc = b"## Plan\n";
        assert!(matches!(
            append_to_section(doc, &Heading::new(2, "Plan"), ""),
            Err(VaultError::InvalidValue { .. })
        ));
    }

    #[test]
    fn stale_hash_is_a_conflict_for_field_and_section() {
        let before = b"---\nA: x\n---\n## Plan\nold\n";
        let after = b"---\nA: changed\n---\n## Plan\nchanged\n";
        let h = Heading::new(2, "Plan");
        assert!(matches!(
            replace_field(after, "A", &scalar("y"), &field_hash(before, "A")),
            Err(VaultError::Conflict { .. })
        ));
        assert!(matches!(
            replace_section(after, &h, "n", &section_hash(before, &h)),
            Err(VaultError::Conflict { .. })
        ));
    }

    #[test]
    fn change_elsewhere_does_not_conflict_and_is_kept() {
        let before = b"---\nA: x\n---\n## Plan\nold\n## Log\nl1\n";
        let after = b"---\nA: x\n---\n## Plan\nold\n## Log\nl1\nl2\n";
        let h = Heading::new(2, "Plan");
        let out = replace_section(after, &h, "new\n", &section_hash(before, &h)).unwrap();
        assert_eq!(s(out), "---\nA: x\n---\n## Plan\nnew\n## Log\nl1\nl2\n");
    }

    #[test]
    fn missing_pieces_are_errors_not_inserts() {
        let doc = b"---\nA: x\n---\n## Plan\nold\n";
        let any = span_hash(b"", Span { start: 0, end: 0 });
        assert!(matches!(
            replace_field(doc, "Nope", &scalar("y"), &any),
            Err(VaultError::FieldNotFound { .. })
        ));
        assert!(matches!(
            replace_field(b"no fm\n", "A", &scalar("y"), &any),
            Err(VaultError::NoFrontmatter)
        ));
        assert!(matches!(
            replace_section(doc, &Heading::new(2, "Nope"), "x", &any),
            Err(VaultError::SectionNotFound { .. })
        ));
        assert!(matches!(
            append_to_section(doc, &Heading::new(2, "Nope"), "x"),
            Err(VaultError::SectionNotFound { .. })
        ));
    }

    #[test]
    fn section_text_that_would_restructure_the_note_is_refused() {
        let doc = b"# Day\n## Plan\na\n### Sub\nb\n## Log\nc\n";
        let plan = Heading::new(2, "Plan");
        let hash = span_hash(doc, find_section(doc, &plan).unwrap());
        for bad in ["x\n## Log\n", "# Top\n", "x\n```\ncode\n", "~~~~\n~~~\n"] {
            assert!(
                matches!(
                    replace_section(doc, &plan, bad, &hash),
                    Err(VaultError::InvalidValue { .. })
                ),
                "replace {bad:?} should be refused"
            );
            assert!(
                matches!(
                    append_to_section(doc, &plan, bad),
                    Err(VaultError::InvalidValue { .. })
                ),
                "append {bad:?} should be refused"
            );
        }
        // Deeper subheadings, headings inside a closed fence and #tags are fine.
        for ok in [
            "### New sub\nx\n",
            "```\n## not a heading\n```\n",
            "#tag line\n",
        ] {
            assert!(
                replace_section(doc, &plan, ok, &hash).is_ok(),
                "replace {ok:?}"
            );
            assert!(append_to_section(doc, &plan, ok).is_ok(), "append {ok:?}");
        }
    }

    #[test]
    fn values_needing_yaml_quoting_or_line_breaks_are_refused() {
        let doc = b"---\nA: x\n---\n";
        let hash = field_hash(doc, "A");
        let bad = [
            "", " lead", "trail ", "a: b", "a #c", "#c", "[x]", "{x}", "&a", "*a", "!a", "|a",
            ">a", "'a", "\"a", "%a", "@a", "`a", "- a", "? a", "-", "?", ",a", "a:", "a\nb",
            "a\rb", "a\tb",
        ];
        for v in bad {
            assert!(
                matches!(
                    replace_field(doc, "A", &scalar(v), &hash),
                    Err(VaultError::InvalidValue { .. })
                ),
                "scalar {v:?} should be refused"
            );
        }
        assert!(matches!(
            replace_field(doc, "A", &list(&["ok", "a: b"]), &hash),
            Err(VaultError::InvalidValue { .. })
        ));
        for v in [
            "Done",
            "In review",
            "a-b",
            "x:y",
            "C#",
            "2026-01-05",
            "zażółć",
        ] {
            assert!(
                replace_field(doc, "A", &scalar(v), &hash).is_ok(),
                "scalar {v:?} should be accepted"
            );
        }
    }

    #[test]
    fn fixture_issue_status_list_round_trips_outside_the_block() {
        let doc = include_bytes!("../../tests/fixtures/vault/Issues/Fake issue 101.md");
        let out = replace_field(
            doc,
            "Status",
            &list(&["Closed"]),
            &field_hash(doc, "Status"),
        )
        .unwrap();
        let out = s(out);
        let orig = String::from_utf8(doc.to_vec()).unwrap();
        assert!(out.contains("Status:\n  - Closed\nLabels:\n  - fake\n  - demo\nPriority: High\n"));
        assert_eq!(
            out.len() + "  - Open\n  - In review\n".len(),
            orig.len() + "  - Closed\n".len()
        );
    }
}

#[cfg(test)]
mod proptests {
    use super::super::markdown::testgen::*;
    use super::*;
    use proptest::prelude::*;

    fn field_value() -> impl Strategy<Value = FieldValue> {
        prop_oneof![
            scalar_value().prop_map(FieldValue::Scalar),
            prop::collection::vec(scalar_value(), 0..4).prop_map(FieldValue::List),
            Just(FieldValue::Empty),
        ]
    }

    fn body_text() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop::sample::select(vec!["text", "- item", "", "#tag", "a b"]),
            0..5,
        )
        .prop_flat_map(|lines| {
            (
                Just(lines),
                prop::sample::select(vec!["\n", "\r\n", "\r"]),
                any::<bool>(),
            )
        })
        .prop_map(|(lines, sep, trailing)| {
            let mut t = lines.join(sep);
            if trailing && !t.is_empty() {
                t.push_str(sep);
            }
            t
        })
    }

    fn only_file_eol(inserted: &[u8], eol: Eol) -> bool {
        match eol {
            Eol::Lf => !inserted.contains(&b'\r'),
            Eol::CrLf => inserted.iter().enumerate().all(|(i, b)| match b {
                b'\n' => i > 0 && inserted[i - 1] == b'\r',
                b'\r' => inserted.get(i + 1) == Some(&b'\n'),
                _ => true,
            }),
        }
    }

    proptest! {
        /// 1. Bytes before span.start and after span.end are the original's, for every edit.
        /// 4. Inserted text contains only the file's EOL.
        #[test]
        fn edits_change_only_the_span_and_use_the_files_eol(
            note in gen_note(),
            value in field_value(),
            body in body_text(),
            append in prop::collection::vec(prop::sample::select(vec!["x", "- y", "z z"]), 1..3),
        ) {
            let bytes = &note.bytes;
            let n = bytes.len();

            // Section replace.
            let span = find_section(bytes, &note.target).unwrap();
            let out = replace_section(bytes, &note.target, &body, &span_hash(bytes, span)).unwrap();
            prop_assert!(out.starts_with(&bytes[..span.start]));
            prop_assert!(out.ends_with(&bytes[span.end..]));
            let ins = &out[span.start..out.len() - (n - span.end)];
            prop_assert!(only_file_eol(ins, note.eol), "inserted {:?}", ins);

            // Section append: insertion lies inside the span, everything else untouched.
            let text = append.join("\n");
            let out = append_to_section(bytes, &note.target, &text).unwrap();
            prop_assert!(out.len() > n);
            prop_assert!(out.starts_with(&bytes[..span.start]));
            prop_assert!(out.ends_with(&bytes[span.end..]));
            let ins = &out[span.start..out.len() - (n - span.end)];
            prop_assert!(only_file_eol(ins, note.eol), "inserted {:?}", ins);
            let want = normalize_eol(&text, note.eol);
            prop_assert!(String::from_utf8_lossy(ins).contains(&want));

            // Field replace on every present key; other fields keep their hashes.
            for (key, _spec, fspan) in &note.fields {
                let out = replace_field(bytes, key, &value, &span_hash(bytes, *fspan)).unwrap();
                prop_assert!(out.starts_with(&bytes[..fspan.start]));
                prop_assert!(out.ends_with(&bytes[fspan.end..]));
                let ins = &out[fspan.start..out.len() - (n - fspan.end)];
                prop_assert!(only_file_eol(ins, note.eol), "inserted {:?}", ins);
                for (other, _, ospan) in &note.fields {
                    if other != key {
                        let h = span_hash(&out, find_field(&out, other).unwrap());
                        prop_assert_eq!(h, span_hash(bytes, *ospan));
                    }
                }
            }
        }

        /// `replace_fields`: bytes outside both field spans are the original's,
        /// for every ordered pair of present fields (edit order = file order
        /// and the reverse, as `set_status` produces when `Done` precedes `Status`).
        #[test]
        fn replace_fields_keeps_every_byte_outside_both_spans(
            note in gen_note(),
            va in field_value(),
            vb in field_value(),
        ) {
            let bytes = &note.bytes;
            for (i, (ka, _, a)) in note.fields.iter().enumerate() {
                for (j, (kb, _, b)) in note.fields.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    let edits = [
                        (*ka, va.clone(), span_hash(bytes, *a)),
                        (*kb, vb.clone(), span_hash(bytes, *b)),
                    ];
                    let out = replace_fields(bytes, &edits).unwrap();
                    let sa = find_field(&out, ka).unwrap();
                    let sb = find_field(&out, kb).unwrap();
                    // Compare in file order, whichever was edited first.
                    let ((s1, o1), (s2, o2)) =
                        if a.start < b.start { ((sa, a), (sb, b)) } else { ((sb, b), (sa, a)) };
                    prop_assert_eq!(&out[..s1.start], &bytes[..o1.start]);
                    prop_assert_eq!(&out[s1.end..s2.start], &bytes[o1.end..o2.start]);
                    prop_assert_eq!(&out[s2.end..], &bytes[o2.end..]);
                    prop_assert!(only_file_eol(sa.slice(&out), note.eol));
                    prop_assert!(only_file_eol(sb.slice(&out), note.eol));
                }
            }
        }

        /// 2. Replacing a section (or a canonical field) with its own current content is a no-op.
        #[test]
        fn replacing_with_current_content_returns_original_bytes(note in gen_note()) {
            let bytes = &note.bytes;
            let span = find_section(bytes, &note.target).unwrap();
            let body = String::from_utf8(span.slice(bytes).to_vec()).unwrap();
            let out = replace_section(bytes, &note.target, &body, &span_hash(bytes, span)).unwrap();
            prop_assert_eq!(&out, bytes);

            for (key, spec, fspan) in &note.fields {
                let value = match spec {
                    FieldSpec::Scalar(v) => FieldValue::Scalar(v.clone()),
                    FieldSpec::List { items, .. } => FieldValue::List(items.clone()),
                };
                let out = replace_field(bytes, key, &value, &span_hash(bytes, *fspan)).unwrap();
                prop_assert_eq!(&out, bytes);
            }
        }

        /// 3 (edit side). Stale hashes always give Conflict and never panic.
        #[test]
        fn stale_hash_always_conflicts(note in gen_note(), body in body_text()) {
            let stale = span_hash(b"something else", Span { start: 0, end: 4 });
            prop_assert!(matches!(
                replace_section(&note.bytes, &note.target, &body, &stale),
                Err(VaultError::Conflict { .. })),
                "expected a conflict"
            );
            for (key, _, _) in &note.fields {
                prop_assert!(matches!(
                    replace_field(&note.bytes, key, &FieldValue::Scalar("v".into()), &stale),
                    Err(VaultError::Conflict { .. })),
                    "expected a conflict"
                );
            }
        }
    }
}
