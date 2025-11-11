#[test]
fn summarize_value_handles_unicode_without_panic() {
    let value = Value::String(
        "非法所得房间abdf sgfsjadlg sadfas百度地方是灯红酒绿 啥地方 ".to_string(),
    );
    let summary = summarize_value(&value);
    assert_eq!(summary, "\"非法所得房间abdf sgfsjadlg sad…\"");
}

#[test]
fn summarize_value_truncates_long_strings_on_char_boundaries() {
    let long = "abcdefghijklmnoabcdefghijklmnoabcdefghijklmno";
    let value = Value::String(long.to_string());
    let summary = summarize_value(&value);
    assert_eq!(summary, "\"abcdefghijklmnoabcdefghi…\"");
}
