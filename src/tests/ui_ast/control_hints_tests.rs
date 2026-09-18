//! `x-control`, `x-slider-marks` and the numeric bounds hints.
//!
//! Two halves: what the hints produce on a well-formed schema, and what they
//! refuse. The refusal half matters as much as the other — a hint that is
//! silently ignored turns a typo into a field that looks fine and behaves
//! wrongly, and the author has no way to tell which happened.

use serde_json::{Value, json};

use crate::ui_ast::{FieldControl, UiNodeKind, build_ui_ast};

/// Build a one-property schema and return the node for that property.
fn node_for(property: Value) -> crate::ui_ast::UiNode {
    let schema = json!({ "type": "object", "properties": { "field": property } });
    let ast = build_ui_ast(&schema).expect("ui ast should build");
    ast.roots
        .into_iter()
        .find(|node| node.pointer == "/field")
        .expect("/field should exist")
}

/// The error message from a schema that should not build.
fn rejection(property: Value) -> String {
    let schema = json!({ "type": "object", "properties": { "field": property } });
    let error = build_ui_ast(&schema).expect_err("this hint should be rejected");
    error.to_string()
}

// ---------------------------------------------------------------- acceptance

#[test]
fn a_number_with_bounds_and_an_explicit_slider_gets_both() {
    let node = node_for(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 100,
        "multipleOf": 5,
        "x-control": "slider"
    }));

    assert_eq!(node.control, Some(FieldControl::Slider));
    let bounds = node.bounds.expect("bounds should be present");
    assert_eq!(bounds.minimum, Some(0.0));
    assert_eq!(bounds.maximum, Some(100.0));
    assert_eq!(bounds.step, Some(5.0));
    assert!(bounds.marks.is_empty());
}

#[test]
fn bounds_are_kept_even_when_no_control_was_named() {
    // The frontend decides whether a slider can be drawn; the parser must not
    // withhold the numbers it would need to decide.
    let node = node_for(json!({ "type": "integer", "minimum": 1, "maximum": 10 }));

    assert_eq!(node.control, None);
    let bounds = node.bounds.expect("bounds should be present");
    assert_eq!(bounds.minimum, Some(1.0));
    assert_eq!(bounds.maximum, Some(10.0));
    assert_eq!(bounds.step, None);
}

#[test]
fn a_node_with_no_numeric_hints_carries_no_bounds() {
    let node = node_for(json!({ "type": "string", "x-control": "text" }));

    assert_eq!(node.control, Some(FieldControl::Text));
    assert!(node.bounds.is_none(), "got {:?}", node.bounds);
}

#[test]
fn a_slider_without_bounds_still_parses() {
    // Deliberate asymmetry with `range`, which does need two declared ends.
    // A slider without bounds is not a schema bug: the frontend falls back to a
    // number box, which is a usable field. A range without bounds has no
    // meaningful fallback, so that one is rejected outright.
    let node = node_for(json!({ "type": "number", "x-control": "slider" }));

    assert_eq!(node.control, Some(FieldControl::Slider));
    assert!(node.bounds.is_none());
}

#[test]
fn marks_alone_are_enough_to_produce_bounds() {
    // Marks carry positions, so a node with only marks still has something to
    // draw on. Whether a track can be drawn is the frontend's call.
    let node = node_for(json!({
        "type": "number",
        "x-slider-marks": [0, 50, 100]
    }));

    let bounds = node.bounds.expect("marks alone should produce bounds");
    assert_eq!(bounds.marks.len(), 3);
    assert_eq!(bounds.minimum, None);
    assert_eq!(bounds.maximum, None);
}

#[test]
fn slider_marks_keep_their_order_and_optional_labels() {
    let node = node_for(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 2,
        "x-control": "slider",
        "x-slider-marks": [
            { "value": 0, "label": "Off" },
            { "value": 1 },
            { "value": 2, "label": "Max" }
        ]
    }));

    let marks = node.bounds.expect("bounds").marks;
    assert_eq!(marks.len(), 3);
    assert_eq!(marks[0].value, 0.0);
    assert_eq!(marks[0].label.as_deref(), Some("Off"));
    // A label is optional: an unlabelled mark is a tick, and the frontend
    // prints the value for it.
    assert_eq!(marks[1].label, None);
    assert_eq!(marks[2].label.as_deref(), Some("Max"));
}

#[test]
fn bare_numbers_and_objects_are_both_accepted_as_marks() {
    // `[0, 50, 100]` is the readable spelling for evenly spaced stops, so it
    // has to keep working alongside the labelled form.
    let node = node_for(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 100,
        "x-slider-marks": [0, { "value": 50, "label": "Half" }, 100]
    }));

    let marks = node.bounds.expect("bounds").marks;
    assert_eq!(marks.len(), 3);
    assert_eq!(marks[0].label, None);
    assert_eq!(marks[1].label.as_deref(), Some("Half"));
    assert_eq!(marks[2].value, 100.0);
}

#[test]
fn every_control_name_the_vocabulary_defines_is_accepted_and_round_trips() {
    let cases: Vec<(&str, FieldControl, Value)> = vec![
        ("text", FieldControl::Text, json!({ "type": "string" })),
        (
            "textarea",
            FieldControl::Textarea,
            json!({ "type": "string" }),
        ),
        ("color", FieldControl::Color, json!({ "type": "string" })),
        (
            "select",
            FieldControl::Select,
            json!({ "type": "string", "enum": ["a", "b"] }),
        ),
        (
            "segmented",
            FieldControl::Segmented,
            json!({ "type": "string", "enum": ["a", "b"] }),
        ),
        (
            "radio",
            FieldControl::Radio,
            json!({ "type": "string", "enum": ["a", "b"] }),
        ),
        ("switch", FieldControl::Switch, json!({ "type": "boolean" })),
        (
            "checkbox",
            FieldControl::Checkbox,
            json!({ "type": "boolean" }),
        ),
        (
            "slider",
            FieldControl::Slider,
            json!({ "type": "number", "minimum": 0, "maximum": 1 }),
        ),
        (
            "range",
            FieldControl::Range,
            json!({
                "type": "array",
                "items": { "type": "number" },
                "minItems": 2,
                "maxItems": 2,
                "minimum": 0,
                "maximum": 10
            }),
        ),
    ];

    for (name, expected, mut property) in cases {
        property["x-control"] = json!(name);
        let node = node_for(property);
        assert_eq!(
            node.control,
            Some(expected),
            "`{name}` parsed to the wrong variant"
        );
        // The keyword is what error messages quote back, so a mismatch here
        // would send an author looking for a name they never wrote.
        assert_eq!(expected.keyword(), name);
    }
}

#[test]
fn a_range_can_be_named_without_bounds() {
    // Bounds are a rendering concern for `range`, not a parse-time one: the
    // frontend declines to draw a track without them and shows the pair as
    // numbers instead. What `range` does require is the two-element shape.
    let node = node_for(json!({
        "type": "array",
        "items": { "type": "number" },
        "minItems": 2,
        "maxItems": 2,
        "x-control": "range"
    }));

    assert_eq!(node.control, Some(FieldControl::Range));
    assert!(node.bounds.is_none());
}

#[test]
fn x_multiline_and_an_explicit_textarea_are_independent_hints() {
    let shorthand = node_for(json!({ "type": "string", "x-multiline": true }));
    let explicit = node_for(json!({ "type": "string", "x-control": "textarea" }));

    assert_eq!(shorthand.control, None, "x-multiline is not x-control");
    let UiNodeKind::Field { multiline, .. } = shorthand.kind else {
        panic!("a string should be a field");
    };
    assert!(multiline);

    let UiNodeKind::Field { multiline, .. } = explicit.kind else {
        panic!("a string should be a field");
    };
    // The explicit control does not set the legacy flag: the frontend resolves
    // `textarea` first, and keeping the two in step would give the same
    // decision two sources of truth.
    assert!(!multiline);
}

#[test]
fn a_control_nested_inside_an_object_is_parsed_too() {
    // Hints are attached by the shared node constructor, so a property that is
    // not a root must get them exactly like one that is.
    let schema = json!({
        "type": "object",
        "properties": {
            "outer": {
                "type": "object",
                "properties": {
                    "inner": { "type": "number", "minimum": 0, "maximum": 9, "x-control": "slider" }
                }
            }
        }
    });

    let ast = build_ui_ast(&schema).expect("ui ast should build");
    let outer = ast
        .roots
        .into_iter()
        .find(|n| n.pointer == "/outer")
        .unwrap();
    let UiNodeKind::Object { children, .. } = outer.kind else {
        panic!("outer should be an object");
    };
    let inner = children
        .into_iter()
        .find(|n| n.pointer == "/outer/inner")
        .expect("inner should exist");

    assert_eq!(inner.control, Some(FieldControl::Slider));
    assert_eq!(inner.bounds.expect("bounds").maximum, Some(9.0));
}

// ---------------------------------------------------------------- rejections

#[test]
fn a_slider_on_a_string_is_rejected() {
    let message = rejection(json!({ "type": "string", "x-control": "slider" }));
    assert!(message.contains("slider"), "got: {message}");
    assert!(
        message.contains("/field"),
        "the pointer should be named: {message}"
    );
}

#[test]
fn a_color_control_on_a_boolean_is_rejected() {
    let message = rejection(json!({ "type": "boolean", "x-control": "color" }));
    assert!(message.contains("color"), "got: {message}");
}

#[test]
fn a_textarea_on_a_number_is_rejected() {
    let message = rejection(json!({ "type": "number", "x-control": "textarea" }));
    assert!(message.contains("textarea"), "got: {message}");
}

#[test]
fn an_enum_control_on_a_plain_string_is_rejected() {
    for control in ["segmented", "radio", "select"] {
        let message = rejection(json!({ "type": "string", "x-control": control }));
        assert!(
            message.contains(control),
            "`{control}` was accepted: {message}"
        );
    }
}

#[test]
fn a_range_on_a_three_element_array_is_rejected() {
    let message = rejection(json!({
        "type": "array",
        "items": { "type": "number" },
        "minItems": 3,
        "maxItems": 3,
        "minimum": 0,
        "maximum": 10,
        "x-control": "range"
    }));
    assert!(message.contains("range"), "got: {message}");
}

#[test]
fn a_range_on_an_unbounded_array_is_rejected() {
    let message = rejection(json!({
        "type": "array",
        "items": { "type": "number" },
        "x-control": "range"
    }));
    assert!(message.contains("range"), "got: {message}");
    // The message has to say how to fix it, not just that it is wrong.
    assert!(message.contains("minItems"), "got: {message}");
}

#[test]
fn a_range_over_strings_is_rejected() {
    let message = rejection(json!({
        "type": "array",
        "items": { "type": "string" },
        "minItems": 2,
        "maxItems": 2,
        "minimum": 0,
        "maximum": 10,
        "x-control": "range"
    }));
    assert!(message.contains("range"), "got: {message}");
}

#[test]
fn a_switch_on_a_number_is_rejected() {
    let message = rejection(json!({ "type": "number", "x-control": "switch" }));
    assert!(message.contains("switch"), "got: {message}");
}

#[test]
fn an_unknown_control_name_is_rejected_rather_than_ignored() {
    // The point of rejecting: a misspelling would otherwise render as a plain
    // input and look like the hint had been applied.
    let message = rejection(json!({ "type": "string", "x-control": "silder" }));
    assert!(message.contains("silder"), "got: {message}");
    // The valid names are listed, so the typo can be corrected without a doc
    // lookup.
    assert!(message.contains("segmented"), "got: {message}");
}

#[test]
fn a_non_string_control_is_rejected() {
    let message = rejection(json!({ "type": "string", "x-control": 42 }));
    assert!(message.contains("x-control"), "got: {message}");
}

#[test]
fn a_control_on_an_object_is_rejected() {
    let message = rejection(json!({
        "type": "object",
        "properties": { "a": { "type": "string" } },
        "x-control": "slider"
    }));
    assert!(message.contains("x-control"), "got: {message}");
}

#[test]
fn non_array_marks_are_rejected() {
    let message = rejection(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 1,
        "x-slider-marks": "nope"
    }));
    assert!(message.contains("x-slider-marks"), "got: {message}");
}

#[test]
fn a_mark_without_a_value_is_rejected() {
    let message = rejection(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 1,
        "x-slider-marks": [{ "label": "x" }]
    }));
    assert!(message.contains("x-slider-marks"), "got: {message}");
}

#[test]
fn a_mark_with_a_non_string_label_is_rejected() {
    let message = rejection(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 1,
        "x-slider-marks": [{ "value": 0, "label": 7 }]
    }));
    assert!(message.contains("label"), "got: {message}");
}

#[test]
fn a_non_numeric_minimum_is_rejected() {
    let message = rejection(json!({
        "type": "number",
        "minimum": "low",
        "maximum": 10,
        "x-control": "slider"
    }));
    assert!(message.contains("minimum"), "got: {message}");
}

#[test]
fn a_non_numeric_multiple_of_is_rejected() {
    let message = rejection(json!({
        "type": "number",
        "minimum": 0,
        "maximum": 10,
        "multipleOf": "five",
        "x-control": "slider"
    }));
    assert!(message.contains("multipleOf"), "got: {message}");
}

#[test]
fn a_non_boolean_x_multiline_is_rejected() {
    let message = rejection(json!({ "type": "string", "x-multiline": "yes" }));
    assert!(message.contains("x-multiline"), "got: {message}");
}
