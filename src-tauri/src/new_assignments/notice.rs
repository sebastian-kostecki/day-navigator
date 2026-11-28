//! Toast text for new assignments. Pure: wording and limits are unit-tested.
//!
//! Only the reference, the title and the kind are read. Never `web_url`, so no
//! host or URL reaches a toast.

use crate::gitlab::{GitlabItem, ItemKind};
use crate::health::{APP_NAME, MAX_NOTICE_LINES, Notice};

/// A title in a separate toast is cut to this many characters.
const SEPARATE_TITLE_CHARS: usize = 100;
/// A title in a line of the combined toast is cut to this many characters.
const COMBINED_TITLE_CHARS: usize = 40;
/// A reference in a line of the combined toast is cut to this many characters,
/// so a deep group path cannot wrap the `i N więcej` line out of view.
const COMBINED_REFERENCE_CHARS: usize = 60;

/// Control characters become spaces; the text is cut to `max` characters on a
/// `char` boundary with `…` appended when it was longer.
fn clean(text: &str, max: usize) -> String {
    let mut chars = text.chars().map(|c| if c.is_control() { ' ' } else { c });
    let mut out: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        out.push('…');
    }
    out
}

fn line(item: &GitlabItem, reference_max: usize, title_max: usize) -> String {
    format!(
        "{} — {}",
        clean(&item.reference, reference_max),
        clean(&item.title, title_max)
    )
}

/// The toasts for the new items of one round, in the order given: none for no
/// items, one each for up to `MAX_NOTICE_LINES`, otherwise one combined toast.
pub fn notices_for(new_items: &[&GitlabItem]) -> Vec<Notice> {
    if new_items.len() <= MAX_NOTICE_LINES {
        return new_items
            .iter()
            .map(|item| Notice {
                title: match item.kind {
                    ItemKind::Issue => format!("{APP_NAME} — nowe zgłoszenie"),
                    ItemKind::MergeRequest if item.review_requested => {
                        format!("{APP_NAME} — nowy MR do review")
                    }
                    ItemKind::MergeRequest => format!("{APP_NAME} — nowy MR"),
                },
                body: line(item, usize::MAX, SEPARATE_TITLE_CHARS),
            })
            .collect();
    }
    let mut lines: Vec<String> = new_items
        .iter()
        .take(MAX_NOTICE_LINES)
        .map(|item| line(item, COMBINED_REFERENCE_CHARS, COMBINED_TITLE_CHARS))
        .collect();
    lines.push(format!("i {} więcej", new_items.len() - MAX_NOTICE_LINES));
    vec![Notice {
        title: format!("{APP_NAME} — nowe przypisania ({})", new_items.len()),
        body: lines.join("\n"),
    }]
}
