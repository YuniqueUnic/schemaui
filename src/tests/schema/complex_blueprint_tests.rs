use serde_json::Value;

use crate::{
    form::FormState,
    schema::{
        blueprint::{build_ui_blueprint, form_schema_blueprint},
        layout::build_form_schema,
    },
};

fn complex_schema() -> Value {
    serde_json::from_str(include_str!("../../../examples/complex.schema.json"))
        .expect("valid schema fixture")
}

#[test]
fn complex_schema_blueprint_exposes_anyof_variants() {
    let schema = complex_schema();
    let form = build_form_schema(&schema).expect("schema parsed");
    let blueprint = form_schema_blueprint(&form);

    let deep_items = find_field(&blueprint, "deepItems").expect("deepItems field");
    let widget = deep_items
        .get("widget")
        .and_then(Value::as_object)
        .expect("deepItems widget");
    assert_eq!(
        widget.get("component"),
        Some(&Value::String("composite_list".into()))
    );
    assert_eq!(widget.get("mode"), Some(&Value::String("anyOf".into())));
    let variants = widget
        .get("variants")
        .and_then(Value::as_array)
        .expect("array variants");
    assert_eq!(
        variants.len(),
        3,
        "deepItems should expose all anyOf variants"
    );
    let titles: Vec<_> = variants
        .iter()
        .filter_map(|variant| variant.get("title").and_then(Value::as_str))
        .collect();
    assert!(
        titles.iter().any(|title| title.contains("string")),
        "string variant should be present: {titles:?}"
    );
    assert!(
        titles.iter().any(|title| title.contains("integer")),
        "integer variant should be present: {titles:?}"
    );
    let object_variant = variants
        .iter()
        .find(|variant| {
            variant
                .get("schema")
                .and_then(|schema| schema.get("properties"))
                .and_then(Value::as_object)
                .is_some_and(|props| props.contains_key("url"))
        })
        .expect("object variant should include $defs/target");
    assert!(
        object_variant
            .get("title")
            .and_then(Value::as_str)
            .map(|label| label.contains("object"))
            .unwrap_or(false),
        "object variant title should describe its shape"
    );

    let options_field = find_field(&blueprint, "options").expect("options field");
    let options_widget = options_field
        .get("widget")
        .and_then(Value::as_object)
        .expect("options widget");
    assert_eq!(
        options_widget.get("component"),
        Some(&Value::String("composite".into()))
    );
    assert_eq!(
        options_widget.get("mode"),
        Some(&Value::String("anyOf".into()))
    );

    // Ensure the blueprint re-serializes via the helper entry point too.
    let pipeline_blueprint = build_ui_blueprint(&schema).expect("pipeline blueprint");
    assert_eq!(
        pipeline_blueprint
            .get("roots")
            .and_then(Value::as_array)
            .map(|roots| roots.len()),
        blueprint
            .get("roots")
            .and_then(Value::as_array)
            .map(|roots| roots.len())
    );

    // Verify we can still build a FormState (schema -> blueprint -> TUI model pipeline).
    let form_state = FormState::from_schema(&form);
    assert!(
        form_state
            .roots
            .iter()
            .any(|root| !root.sections.is_empty()),
        "form state should include focusable sections"
    );
}

fn find_field<'a>(blueprint: &'a Value, name: &str) -> Option<&'a Value> {
    let roots = blueprint.get("roots")?.as_array()?;
    for root in roots {
        if let Some(field) = find_field_in_sections(root.get("sections")?, name) {
            return Some(field);
        }
    }
    None
}

fn find_field_in_sections<'a>(sections: &'a Value, name: &str) -> Option<&'a Value> {
    let sections = sections.as_array()?;
    for section in sections {
        if let Some(fields) = section.get("fields").and_then(Value::as_array) {
            for field in fields {
                if field.get("name").and_then(Value::as_str) == Some(name) {
                    return Some(field);
                }
            }
        }
        if let Some(children) = section.get("children") {
            if let Some(found) = find_field_in_sections(children, name) {
                return Some(found);
            }
        }
    }
    None
}
