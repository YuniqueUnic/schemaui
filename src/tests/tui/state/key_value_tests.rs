use crate::tui::state::value_summary::summarize_value;
use serde_json::{Value, json};

#[test]
fn summarize_value_handles_unicode_without_panic() {
    let value =
        Value::String("非法所得房间 abdf sgfsjadlg sadfas 百度地方是灯红酒绿 啥地方 ".to_string());
    let summary = summarize_value(&value);
    assert_eq!(summary, "\"非法所得房间 abdf sgfsjadlg sa…\"");
}

#[test]
fn summarize_value_truncates_long_strings_on_char_boundaries() {
    let long = "abcdefghijklmnoabcdefghijklmnoabcdefghijklmno";
    let value = Value::String(long.to_string());
    let summary = summarize_value(&value);
    assert_eq!(summary, "\"abcdefghijklmnoabcdefghi…\"");
}

#[test]
fn summarize_value_renders_object_preview() {
    let value = json!({
        "Bar": "on tui",
        "Id": 0
    });
    let summary = summarize_value(&value);
    assert_eq!(summary, "{ Bar: on tui, Id: 0 }");
}
