//! The visibility filter for the "Praca (GitLab)" list: which assigned items
//! the view shows. Pure code. The filter narrows only what is shown; every
//! other consumer keeps the full list.
//!
//! Privacy: entries are work data (project paths, labels, milestone titles),
//! so `Debug` is opaque and nothing here formats a value. The values leave the
//! module only through serde, to the UI.

use std::fmt;

use serde::Serialize;

use super::{GitlabItem, ItemKind};

/// Include and exclude entries of one dimension, lowercased. An empty list is
/// no constraint.
#[derive(Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionRule {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl fmt::Debug for DimensionRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DimensionRule(..)")
    }
}

impl DimensionRule {
    fn is_active(&self) -> bool {
        !self.include.is_empty() || !self.exclude.is_empty()
    }

    /// `value` is `None` when the item has no value in this dimension: such an
    /// item fails a non-empty `include` and is never removed by `exclude`.
    fn allows(&self, value: Option<&str>, matches: impl Fn(&str, &str) -> bool) -> bool {
        let Some(value) = value else {
            return self.include.is_empty();
        };
        if self.exclude.iter().any(|e| matches(value, e)) {
            return false;
        }
        self.include.is_empty() || self.include.iter().any(|e| matches(value, e))
    }
}

/// Which assigned items the list shows, and whether recently closed ones are
/// listed too (`closed_days`, off by default).
#[derive(Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitlabFilter {
    pub projects: DimensionRule,
    pub labels: DimensionRule,
    pub milestones: DimensionRule,
    pub closed_days: Option<u32>,
}

impl fmt::Debug for GitlabFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GitlabFilter(..)")
    }
}

/// The project path of an item: the part of `reference` before the last `#`
/// (issue) or `!` (merge request). `None` when the separator is missing or the
/// path is empty.
pub fn project_path(item: &GitlabItem) -> Option<&str> {
    let separator = match item.kind {
        ItemKind::Issue => '#',
        ItemKind::MergeRequest => '!',
    };
    let (path, _) = item.reference.rsplit_once(separator)?;
    if path.is_empty() { None } else { Some(path) }
}

/// `entry` (lowercase) equals `path` or is a group prefix of it on a `/` boundary.
fn project_matches(path: &str, entry: &str) -> bool {
    match path.strip_prefix(entry) {
        Some("") => true,
        Some(rest) => rest.starts_with('/'),
        None => false,
    }
}

impl GitlabFilter {
    /// True when any include or exclude list is non-empty. `closed_days` alone
    /// does not narrow the open list.
    pub fn is_active(&self) -> bool {
        self.projects.is_active() || self.labels.is_active() || self.milestones.is_active()
    }

    /// Whether the view shows `item`. Dimensions combine with AND; matching is
    /// case-insensitive.
    pub fn keeps(&self, item: &GitlabItem) -> bool {
        let path = project_path(item).map(str::to_lowercase);
        if !self.projects.allows(path.as_deref(), project_matches) {
            return false;
        }
        let milestone = item.milestone.as_deref().map(str::to_lowercase);
        if !self
            .milestones
            .allows(milestone.as_deref(), |value, entry| value == entry)
        {
            return false;
        }
        self.labels_allow(item)
    }

    fn labels_allow(&self, item: &GitlabItem) -> bool {
        let labels: Vec<String> = item.label_names().map(str::to_lowercase).collect();
        if labels
            .iter()
            .any(|l| self.labels.exclude.iter().any(|e| e == l))
        {
            return false;
        }
        self.labels.include.is_empty()
            || labels
                .iter()
                .any(|l| self.labels.include.iter().any(|e| e == l))
    }

    /// Splits `items` into the shown ones (input order kept) and the number removed.
    pub fn apply(&self, items: &[GitlabItem]) -> (Vec<GitlabItem>, u32) {
        let kept: Vec<GitlabItem> = items.iter().filter(|i| self.keeps(i)).cloned().collect();
        let removed = (items.len() - kept.len()) as u32;
        (kept, removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gitlab::GitlabLabel;

    fn item(kind: ItemKind, reference: &str) -> GitlabItem {
        GitlabItem {
            kind,
            id: 1,
            project_id: 1,
            iid: 1,
            reference: reference.to_string(),
            title: "T".into(),
            web_url: None,
            updated_at: "2026-03-02T10:00:00Z".into(),
            labels: vec![],
            draft: false,
            changed: None,
            milestone: None,
            is_new: false,
            hidden_at_ms: None,
            review_requested: false,
        }
    }

    fn issue(reference: &str) -> GitlabItem {
        item(ItemKind::Issue, reference)
    }

    fn with_labels(mut i: GitlabItem, labels: &[&str]) -> GitlabItem {
        i.labels = labels
            .iter()
            .map(|l| GitlabLabel {
                name: l.to_string(),
                color: None,
                text_color: None,
            })
            .collect();
        i
    }

    fn with_milestone(mut i: GitlabItem, title: &str) -> GitlabItem {
        i.milestone = Some(title.to_string());
        i
    }

    fn rule(include: &[&str], exclude: &[&str]) -> DimensionRule {
        DimensionRule {
            include: include.iter().map(|s| s.to_string()).collect(),
            exclude: exclude.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn projects(include: &[&str], exclude: &[&str]) -> GitlabFilter {
        GitlabFilter {
            projects: rule(include, exclude),
            ..GitlabFilter::default()
        }
    }

    fn labels(include: &[&str], exclude: &[&str]) -> GitlabFilter {
        GitlabFilter {
            labels: rule(include, exclude),
            ..GitlabFilter::default()
        }
    }

    fn milestones(include: &[&str], exclude: &[&str]) -> GitlabFilter {
        GitlabFilter {
            milestones: rule(include, exclude),
            ..GitlabFilter::default()
        }
    }

    #[test]
    fn default_filter_is_inactive_and_keeps_everything() {
        let f = GitlabFilter::default();
        assert!(!f.is_active());
        assert!(f.keeps(&issue("g/p#1")));
        assert!(f.keeps(&issue("no-separator")));
        let closed_only = GitlabFilter {
            closed_days: Some(7),
            ..GitlabFilter::default()
        };
        assert!(!closed_only.is_active());
        for f in [
            projects(&["a"], &[]),
            projects(&[], &["a"]),
            labels(&["a"], &[]),
            labels(&[], &["a"]),
            milestones(&["a"], &[]),
            milestones(&[], &["a"]),
        ] {
            assert!(f.is_active());
        }
    }

    #[test]
    fn include_matches_any_entry() {
        let f = projects(&["acme/one", "acme/two"], &[]);
        assert!(f.keeps(&issue("acme/one#1")));
        assert!(f.keeps(&issue("acme/two#1")));
        assert!(!f.keeps(&issue("acme/three#1")));
        let f = labels(&["red", "blue"], &[]);
        assert!(f.keeps(&with_labels(issue("g/p#1"), &["x", "blue"])));
        assert!(!f.keeps(&with_labels(issue("g/p#1"), &["x"])));
        assert!(!f.keeps(&issue("g/p#1")));
    }

    #[test]
    fn label_colours_do_not_change_what_the_filter_keeps() {
        let plain = with_labels(issue("g/p#1"), &["Blue", "wip"]);
        let mut coloured = plain.clone();
        for l in &mut coloured.labels {
            l.color = Some("#0000ff".to_string());
            l.text_color = Some("#ffffff".to_string());
        }
        for f in [
            labels(&["blue"], &[]),
            labels(&[], &["wip"]),
            labels(&["red"], &[]),
            labels(&[], &["red"]),
        ] {
            assert_eq!(f.keeps(&plain), f.keeps(&coloured));
        }
        assert!(labels(&["blue"], &[]).keeps(&coloured));
        assert!(!labels(&[], &["wip"]).keeps(&coloured));
    }

    #[test]
    fn exclude_removes_on_any_entry_and_wins_over_include() {
        let f = labels(&[], &["blocked", "wip"]);
        assert!(!f.keeps(&with_labels(issue("g/p#1"), &["ok", "wip"])));
        assert!(f.keeps(&with_labels(issue("g/p#1"), &["ok"])));
        let f = labels(&["ok"], &["blocked"]);
        assert!(!f.keeps(&with_labels(issue("g/p#1"), &["ok", "blocked"])));
        assert!(f.keeps(&with_labels(issue("g/p#1"), &["ok"])));
        let f = projects(&["acme"], &["acme/legacy"]);
        assert!(f.keeps(&issue("acme/api#1")));
        assert!(!f.keeps(&issue("acme/legacy/old#1")));
    }

    #[test]
    fn dimensions_combine_with_and() {
        let f = GitlabFilter {
            projects: rule(&["acme"], &[]),
            labels: rule(&["red"], &[]),
            milestones: rule(&["sprint 1"], &[]),
            closed_days: None,
        };
        let good = with_milestone(with_labels(issue("acme/p#1"), &["red"]), "Sprint 1");
        assert!(f.keeps(&good));
        let mut wrong_project = good.clone();
        wrong_project.reference = "other/p#1".into();
        let mut wrong_label = good.clone();
        wrong_label.labels = with_labels(issue("acme/p#1"), &["blue"]).labels;
        let mut wrong_milestone = good.clone();
        wrong_milestone.milestone = Some("Sprint 2".into());
        for i in [wrong_project, wrong_label, wrong_milestone] {
            assert!(!f.keeps(&i));
        }
    }

    #[test]
    fn project_entry_matches_exactly_or_as_group_prefix_on_a_slash() {
        let f = projects(&["acme/backend"], &[]);
        assert!(f.keeps(&issue("acme/backend#7")));
        assert!(f.keeps(&issue("acme/backend/api#4")));
        assert!(f.keeps(&item(ItemKind::MergeRequest, "acme/backend/deep/er!2")));
        assert!(!f.keeps(&issue("acme/backend-old#1")));
        assert!(!f.keeps(&issue("acme/backendx/api#1")));
        assert!(!f.keeps(&issue("acme#1")));
        assert!(!f.keeps(&issue("other/acme/backend#1")));
    }

    #[test]
    fn matching_is_case_insensitive() {
        let f = projects(&["acme/backend"], &[]);
        assert!(f.keeps(&issue("ACME/Backend/Api#1")));
        let f = labels(&["needs review"], &[]);
        assert!(f.keeps(&with_labels(issue("g/p#1"), &["Needs Review"])));
        let f = milestones(&["sprint 42"], &[]);
        assert!(f.keeps(&with_milestone(issue("g/p#1"), "Sprint 42")));
        let f = milestones(&[], &["sprint 42"]);
        assert!(!f.keeps(&with_milestone(issue("g/p#1"), "SPRINT 42")));
    }

    #[test]
    fn milestone_is_exact_not_a_prefix() {
        let f = milestones(&["sprint 4"], &[]);
        assert!(!f.keeps(&with_milestone(issue("g/p#1"), "Sprint 42")));
        assert!(f.keeps(&with_milestone(issue("g/p#1"), "Sprint 4")));
    }

    #[test]
    fn no_milestone_fails_include_and_is_never_excluded() {
        let none = issue("g/p#1");
        assert!(!milestones(&["sprint 1"], &[]).keeps(&none));
        assert!(milestones(&[], &["sprint 1"]).keeps(&none));
        assert!(!milestones(&["sprint 1"], &["sprint 2"]).keeps(&none));
        assert!(milestones(&[], &[]).keeps(&none));
    }

    #[test]
    fn unreadable_project_path_fails_include_and_is_never_excluded() {
        for reference in ["noseparator", "#5", "!5", "g/p!5"] {
            let i = issue(reference);
            assert_eq!(project_path(&i), None, "{reference}");
            assert!(!projects(&["g"], &[]).keeps(&i), "{reference}");
            assert!(projects(&[], &["g"]).keeps(&i), "{reference}");
            assert!(projects(&[], &[]).keeps(&i), "{reference}");
        }
    }

    #[test]
    fn project_path_uses_the_separator_of_the_kind() {
        assert_eq!(project_path(&issue("a/b/c#12")), Some("a/b/c"));
        assert_eq!(
            project_path(&item(ItemKind::MergeRequest, "a/b/c!34")),
            Some("a/b/c")
        );
        // The other kind's separator is not a separator.
        assert_eq!(project_path(&issue("a/b!34")), None);
        assert_eq!(project_path(&item(ItemKind::MergeRequest, "a/b#12")), None);
        // The last separator wins.
        assert_eq!(project_path(&issue("a#b/c#1")), Some("a#b/c"));
    }

    #[test]
    fn filter_applies_to_merge_requests_like_issues() {
        let f = projects(&["acme"], &[]);
        assert!(f.keeps(&item(ItemKind::MergeRequest, "acme/p!3")));
        assert!(!f.keeps(&item(ItemKind::MergeRequest, "other/p!3")));
    }

    #[test]
    fn apply_keeps_order_and_counts_removed() {
        let f = projects(&["acme"], &[]);
        let items = vec![
            issue("acme/a#1"),
            issue("other/b#2"),
            issue("acme/c#3"),
            issue("other/d#4"),
            issue("acme/e#5"),
        ];
        let (kept, removed) = f.apply(&items);
        let refs: Vec<&str> = kept.iter().map(|i| i.reference.as_str()).collect();
        assert_eq!(refs, ["acme/a#1", "acme/c#3", "acme/e#5"]);
        assert_eq!(removed, 2);

        let (kept, removed) = GitlabFilter::default().apply(&items);
        assert_eq!(kept, items);
        assert_eq!(removed, 0);

        let (kept, removed) = f.apply(&[]);
        assert!(kept.is_empty());
        assert_eq!(removed, 0);
    }

    #[test]
    fn debug_never_shows_values() {
        let f = GitlabFilter {
            projects: rule(&["secret/project"], &[]),
            labels: rule(&[], &["secret-label"]),
            milestones: rule(&["secret milestone"], &[]),
            closed_days: Some(3),
        };
        let shown = format!("{f:?} {:?}", f.projects);
        assert!(!shown.contains("secret"), "{shown}");
        assert_eq!(format!("{f:?}"), "GitlabFilter(..)");
    }
}
