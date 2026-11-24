//! Unit tests with invented data.

use super::*;

fn t(id: SourceId, from: Option<&str>, to: Option<&str>) -> Transition {
    Transition {
        id,
        label: "Źródło".to_string(),
        from_kind: from.map(str::to_string),
        to_kind: to.map(str::to_string),
    }
}

#[test]
fn entering_an_error_becomes_source_error() {
    let ev = from_transitions(&[t(SourceId::Gitlab, None, Some("LoggedOut"))]);
    assert_eq!(
        ev,
        vec![NewEvent::SourceError {
            source: SourceId::Gitlab,
            label: "Źródło".to_string(),
            error_kind: "LoggedOut".to_string(),
        }]
    );
}

#[test]
fn changing_the_error_kind_becomes_source_error() {
    let ev = from_transitions(&[t(SourceId::Github, Some("Failed"), Some("LoggedOut"))]);
    assert!(matches!(
        &ev[0],
        NewEvent::SourceError { error_kind, .. } if error_kind == "LoggedOut"
    ));
}

#[test]
fn recovery_becomes_source_recovered() {
    let ev = from_transitions(&[t(SourceId::AppData, Some("Busy"), None)]);
    assert_eq!(
        ev,
        vec![NewEvent::SourceRecovered {
            source: SourceId::AppData,
            label: "Źródło".to_string(),
        }]
    );
    assert!(from_transitions(&[]).is_empty());
}

#[test]
fn title_is_cut_on_a_character_boundary() {
    let long: String = "żółć".repeat(100);
    let cut = cut(&long);
    assert_eq!(cut.chars().count(), 300);
    assert!(long.starts_with(&cut));
    assert_eq!(super::cut("krótki"), "krótki");
}

#[test]
fn item_json_has_kind_and_camel_case_fields() {
    let item = EventItem {
        id: 3,
        at: "2026-11-23T10:00:00".to_string(),
        event: EventView::Assigned {
            item_kind: ItemKind::MergeRequest,
            reference: "acme/web!421".to_string(),
            title: "Wymyślony tytuł".to_string(),
        },
    };
    let v = serde_json::to_value(&item).unwrap();
    assert_eq!(v["id"], 3);
    assert_eq!(v["at"], "2026-11-23T10:00:00");
    assert_eq!(v["kind"], "assigned");
    assert_eq!(v["itemKind"], "mergeRequest");
    assert_eq!(v["reference"], "acme/web!421");

    let item = EventItem {
        id: 4,
        at: "2026-11-23T10:01:00".to_string(),
        event: EventView::Ticked(Subject {
            source: ItemSource::Own,
            reference: None,
            title: "Wymyślony tytuł".to_string(),
        }),
    };
    let v = serde_json::to_value(&item).unwrap();
    assert_eq!(v["kind"], "ticked");
    assert_eq!(v["source"], "own");
    assert!(v["reference"].is_null());

    let item = EventItem {
        id: 5,
        at: "2026-11-23T10:02:00".to_string(),
        event: EventView::SourceError {
            source_id: SourceId::AppData,
            label: "Dane aplikacji".to_string(),
            reason: "plik danych zajęty".to_string(),
        },
    };
    let v = serde_json::to_value(&item).unwrap();
    assert_eq!(v["kind"], "sourceError");
    assert_eq!(v["sourceId"], "app-data");
    assert_eq!(v["reason"], "plik danych zajęty");

    let v = serde_json::to_value(EventItem {
        id: 6,
        at: "x".to_string(),
        event: EventView::WorkStarted,
    })
    .unwrap();
    assert_eq!(v["kind"], "workStarted");
}
