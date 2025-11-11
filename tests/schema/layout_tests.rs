#[test]
fn builds_nested_sections_and_general_root() {
    let schema = json!({
        "type": "object",
        "definitions": {
            "duration": {
                "type": "object",
                "properties": {
                    "value": {"type": "integer"}
                }
            }
        },
        "properties": {
            "metadata": {
                "type": "object",
                "properties": {
                    "serviceName": {"type": "string"}
                }
            },
            "runtime": {
                "type": "object",
                "properties": {
                    "http": {
                        "type": "object",
                        "properties": {
                            "port": {"type": "integer"},
                            "limits": {
                                "type": "object",
                                "properties": {
                                    "timeout": {"$ref": "#/definitions/duration"}
                                }
                            }
                        }
                    }
                }
            },
            "generalFlag": {"type": "string"}
        }
    });

    let form = build_form_schema(&schema).expect("schema parsed");
    assert_eq!(form.roots.len(), 3);
    let runtime = form
        .roots
        .iter()
        .find(|root| root.id == "runtime")
        .expect("runtime root");
    assert_eq!(runtime.sections.len(), 1);
    let http = &runtime.sections[0];
    assert_eq!(http.children.len(), 1);
    let http_child = &http.children[0];
    assert_eq!(http_child.fields.len(), 1);
}

#[test]
fn composite_variants_keep_definitions() {
    let schema = json!({
        "type": "object",
        "definitions": {
            "endpoint": {
                "type": "object",
                "properties": {
                    "url": {"type": "string"}
                }
            }
        },
        "properties": {
            "notifications": {
                "type": "object",
                "properties": {
                    "channel": {
                        "oneOf": [
                            {
                                "title": "HTTP",
                                "properties": {
                                    "type": {"const": "http"},
                                    "target": {"$ref": "#/definitions/endpoint"}
                                },
                                "required": ["type", "target"]
                            }
                        ]
                    }
                }
            }
        }
    });

    let form = build_form_schema(&schema).expect("schema parsed");
    let notifications = form
        .roots
        .iter()
        .find(|root| root.id == "notifications")
        .expect("notifications root");
    let section = notifications.sections.first().expect("section");
    let channel = section
        .fields
        .iter()
        .find(|field| field.name == "channel")
        .expect("channel field");
    match &channel.kind {
        FieldKind::Composite(composite) => {
            assert!(
                composite
                    .variants
                    .first()
                    .and_then(|variant| variant.schema.get("definitions"))
                    .is_some(),
                "variant schema should embed definitions"
            );
        }
        other => panic!("expected composite field, got {:?}", other),
    }
}

#[test]
fn notifications_sections_do_not_duplicate_parent_field() {
    let schema = json!({
        "type": "object",
        "properties": {
            "notifications": {
                "type": "object",
                "properties": {
                    "channels": {"type": "array", "items": {"type": "string"}},
                    "templates": {
                        "type": "object",
                        "additionalProperties": {"type": "string"}
                    }
                },
                "additionalProperties": false
            }
        }
    });
    let form = build_form_schema(&schema).expect("schema parsed");
    let notifications = form
        .roots
        .iter()
        .find(|root| root.id == "notifications")
        .expect("notifications root");
    let section = notifications.sections.first().expect("section");
    let names: Vec<_> = section
        .fields
        .iter()
        .map(|field| field.name.clone())
        .collect();
    assert_eq!(names, vec!["channels", "templates"]);
}

#[test]
fn additional_properties_true_does_not_add_phantom_field() {
    let schema = json!({
        "type": "object",
        "properties": {
            "room": {
                "type": "object",
                "properties": {
                    "max_size": {"type": "integer"}
                },
                "additionalProperties": true
            }
        }
    });
    let form = build_form_schema(&schema).expect("schema parsed");
    let room = form
        .roots
        .iter()
        .find(|root| root.id == "room")
        .expect("room root");
    let section = room.sections.first().expect("room section");
    assert!(
        section.fields.iter().all(|field| field.name != "room"),
        "room section should only contain actual child fields"
    );
    let names: Vec<_> = section.fields.iter().map(|field| field.name.clone()).collect();
    assert_eq!(names, vec!["max_size"]);
}

#[test]
fn pattern_properties_become_key_value_fields() {
    let schema = json!({
        "type": "object",
        "properties": {
            "labels": {
                "type": "object",
                "patternProperties": {
                    "^[a-z]+$": {"type": "string"}
                },
                "additionalProperties": false
            }
        }
    });
    let form = build_form_schema(&schema).expect("schema parsed");
    let field = find_field(&form, |field| field.name == "labels").expect("labels field");
    assert!(matches!(field.kind, FieldKind::KeyValue(_)));
}

#[test]
fn arrays_without_item_schema_fallback_to_json_array() {
    let schema = json!({
        "type": "object",
        "properties": {
            "expose_headers": {
                "type": "array",
                "description": "headers exposed via CORS"
            }
        }
    });
    let form = build_form_schema(&schema).expect("schema parsed");
    let field = find_field(&form, |field| field.name == "expose_headers")
        .expect("expose_headers field");
    match &field.kind {
        FieldKind::Array(inner) => assert!(matches!(inner.as_ref(), FieldKind::Json)),
        other => panic!("expected array kind, got {:?}", other),
    }
}

#[test]
fn multi_level_refs_are_resolved() {
    let schema = json!({
        "type": "object",
        "definitions": {
            "duration": {
                "type": "object",
                "properties": {
                    "value": {"type": "integer"}
                }
            },
            "timeout": {
                "$ref": "#/definitions/duration"
            }
        },
        "properties": {
            "runtime": {
                "type": "object",
                "properties": {
                    "limits": {
                        "type": "object",
                        "properties": {
                            "requestTimeout": {"$ref": "#/definitions/timeout"}
                        }
                    }
                }
            }
        }
    });
    let form = build_form_schema(&schema).expect("schema parsed");
    let field = find_field(&form, |field| {
        field
            .pointer
            .ends_with("/runtime/limits/requestTimeout/value")
    })
    .expect("requestTimeout value field");
    assert_eq!(field.name, "value");
}

fn find_field<'a>(
    form: &'a FormSchema,
    predicate: impl Fn(&FieldSchema) -> bool,
) -> Option<&'a FieldSchema> {
    for root in &form.roots {
        if let Some(field) = find_in_sections(&root.sections, &predicate) {
            return Some(field);
        }
    }
    None
}

fn find_in_sections<'a>(
    sections: &'a [FormSection],
    predicate: &impl Fn(&FieldSchema) -> bool,
) -> Option<&'a FieldSchema> {
    for section in sections {
        for field in &section.fields {
            if predicate(field) {
                return Some(field);
            }
        }
        if let Some(found) = find_in_sections(&section.children, predicate) {
            return Some(found);
        }
    }
    None
}
