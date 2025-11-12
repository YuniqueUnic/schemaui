#[test]
fn formats_collection_status_with_selection() {
    let text = super::format_collection_value(
        "Map",
        2,
        Some("key=value".to_string()),
        "(Ctrl+E edit)",
    );
    assert_eq!(text, "Map[2] • key=value (Ctrl+E edit)");
}

#[test]
fn formats_collection_status_when_empty() {
    let text = super::format_collection_value("List", 0, None, "(Ctrl+N add)");
    assert_eq!(text, "List: empty (Ctrl+N add)");
}
