use schemars::schema::RootSchema;
use serde_json::json;

#[test]
fn resolves_definition_reference() {
    let raw = json!({
        "definitions": {
            "duration": {
                "type": "object",
                "properties": {
                    "value": {"type": "integer"}
                }
            }
        },
        "type": "object",
        "properties": {
            "timeout": {"$ref": "#/definitions/duration"}
        }
    });
    let root: RootSchema = serde_json::from_value(raw.clone()).expect("valid root schema");
    let resolver = SchemaResolver::new(&raw, &root);
    let timeout_schema = root
        .schema
        .object
        .as_ref()
        .unwrap()
        .properties
        .get("timeout")
        .unwrap();
    let resolved = resolver
        .resolve_schema(timeout_schema)
        .expect("resolution succeeds");
    assert!(resolved.object.is_some());
    assert!(resolved
        .object
        .as_ref()
        .unwrap()
        .properties
        .contains_key("value"));
}

#[test]
fn resolves_pointer_reference() {
    let raw = json!({
        "type": "object",
        "properties": {
            "base": {
                "type": "object",
                "properties": {
                    "url": {"type": "string"}
                }
            },
            "clone": {"$ref": "#/properties/base"}
        }
    });
    let root: RootSchema = serde_json::from_value(raw.clone()).expect("valid root schema");
    let resolver = SchemaResolver::new(&raw, &root);
    let clone_schema = root
        .schema
        .object
        .as_ref()
        .unwrap()
        .properties
        .get("clone")
        .unwrap();
    let resolved = resolver
        .resolve_schema(clone_schema)
        .expect("resolution succeeds");
    let object = resolved.object.expect("object schema");
    assert!(object.properties.contains_key("url"));
}
