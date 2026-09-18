use crate::tui::view::{
    UiContext,
    components::footer::{
        FooterActionItem, FooterStatusTone, footer_action_items, footer_status_model,
    },
};

#[test]
fn footer_action_items_split_help_snippets_into_pairs() {
    let items = footer_action_items(Some(
        "Ctrl+S -> validate and save • Ctrl+Q -> quit • Tab -> next field",
    ));

    assert_eq!(
        items,
        vec![
            FooterActionItem {
                keys: "Ctrl+S".to_string(),
                action: "validate and save".to_string(),
            },
            FooterActionItem {
                keys: "Ctrl+Q".to_string(),
                action: "quit".to_string(),
            },
            FooterActionItem {
                keys: "Tab".to_string(),
                action: "next field".to_string(),
            },
        ]
    );
}

#[test]
fn footer_status_model_prioritizes_error_tone_and_focus_context() {
    let ctx = UiContext {
        status_message: "3 issue(s) remaining",
        dirty: true,
        error_count: 3,
        help: None,
        global_errors: &[String::from(
            "matrix[0][2] invalid: expected boolean or null but got string that keeps growing",
        )],
        focus_label: Some("Level4 › Matrix".to_string()),
        session_title: Some("Service Config"),
        popup: None,
        composite_overlay: None,
        help_overlay: None,
        time_remaining: None,
    };

    let status = footer_status_model(&ctx);

    assert_eq!(status.tone, FooterStatusTone::Error);
    assert_eq!(status.badge, "ERR 3");
    assert_eq!(status.message, "3 issue(s) remaining");
    assert!(
        status.meta.iter().any(|entry| entry == "Unsaved changes"),
        "dirty state should still be surfaced when errors exist"
    );
    assert!(
        status
            .meta
            .iter()
            .any(|entry| entry == "Focus Level4 › Matrix"),
        "focus label should remain visible in the footer meta line"
    );
    assert!(
        status
            .alert
            .as_deref()
            .is_some_and(|alert| alert.ends_with('…')),
        "global alert text should be truncated for readability"
    );
    assert_eq!(status.session_title.as_deref(), Some("Service Config"));
}

#[test]
fn footer_status_model_compacts_ready_message() {
    let ctx = UiContext {
        status_message: crate::tui::app::status::READY_STATUS,
        dirty: false,
        error_count: 0,
        help: Some("Ctrl+S -> validate and save"),
        global_errors: &[],
        focus_label: None,
        session_title: Some("SchemaUI Demo"),
        popup: None,
        composite_overlay: None,
        help_overlay: None,
        time_remaining: None,
    };

    let status = footer_status_model(&ctx);

    assert_eq!(status.tone, FooterStatusTone::Ready);
    assert_eq!(status.badge, "READY");
    assert_eq!(status.message, "Ready for input");
    assert!(status.meta.is_empty());
    assert!(status.alert.is_none());
    assert_eq!(status.session_title.as_deref(), Some("SchemaUI Demo"));
}

#[test]
fn footer_status_model_surfaces_the_timeout_countdown_first() {
    let ctx = UiContext {
        status_message: "Ready for input",
        dirty: false,
        error_count: 0,
        help: None,
        global_errors: &[],
        focus_label: Some("General › Name".to_string()),
        session_title: Some("SchemaUI Demo"),
        popup: None,
        composite_overlay: None,
        help_overlay: None,
        time_remaining: Some(std::time::Duration::from_secs(90)),
    };

    let status = footer_status_model(&ctx);

    // First, so the deadline is read before the incidental focus hint.
    assert_eq!(
        status.meta.first().map(String::as_str),
        Some("Timeout in 1:30")
    );
    assert!(
        status
            .meta
            .iter()
            .any(|entry| entry == "Focus General › Name"),
        "the countdown must not crowd out the focus label"
    );
}

#[test]
fn footer_status_model_widens_the_countdown_past_an_hour() {
    let ctx = UiContext {
        status_message: "Ready for input",
        dirty: false,
        error_count: 0,
        help: None,
        global_errors: &[],
        focus_label: None,
        session_title: None,
        popup: None,
        composite_overlay: None,
        help_overlay: None,
        time_remaining: Some(std::time::Duration::from_secs(3_661)),
    };

    let status = footer_status_model(&ctx);

    assert_eq!(status.meta, vec!["Timeout in 1:01:01".to_string()]);
}

#[test]
fn footer_status_model_shows_no_countdown_for_an_unbounded_session() {
    let ctx = UiContext {
        status_message: "Ready for input",
        dirty: false,
        error_count: 0,
        help: None,
        global_errors: &[],
        focus_label: None,
        session_title: None,
        popup: None,
        composite_overlay: None,
        help_overlay: None,
        time_remaining: None,
    };

    let status = footer_status_model(&ctx);

    assert!(status.meta.is_empty(), "got {:?}", status.meta);
}

#[test]
fn footer_status_model_rounds_the_countdown_up() {
    // Sub-second remainders must not read as 0:00 while the session is still
    // open, and a fresh `--timeout 4` must open on 0:04 rather than 0:03.
    let label = |millis: u64| {
        let ctx = UiContext {
            status_message: "Ready for input",
            dirty: false,
            error_count: 0,
            help: None,
            global_errors: &[],
            focus_label: None,
            session_title: None,
            popup: None,
            composite_overlay: None,
            help_overlay: None,
            time_remaining: Some(std::time::Duration::from_millis(millis)),
        };
        footer_status_model(&ctx)
            .meta
            .first()
            .cloned()
            .unwrap_or_default()
    };

    assert_eq!(label(4_000), "Timeout in 0:04");
    assert_eq!(label(3_999), "Timeout in 0:04");
    assert_eq!(label(1_001), "Timeout in 0:02");
    assert_eq!(label(1_000), "Timeout in 0:01");
    assert_eq!(label(1), "Timeout in 0:01");
    assert_eq!(label(0), "Timeout in 0:00");
}
