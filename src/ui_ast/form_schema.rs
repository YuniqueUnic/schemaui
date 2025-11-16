use std::collections::HashMap;

use crate::domain::{
    CompositeField, CompositeMode as DomainCompositeMode, CompositeVariant, FieldKind, FieldSchema,
    FormSchema, FormSection, RootSection,
};

use super::{CompositeMode, ScalarKind, UiAst, UiNode, UiNodeKind, UiVariant};

/// Build a legacy `FormSchema` tree from the canonical [`UiAst`].
pub fn form_schema_from_ui_ast(ast: &UiAst) -> FormSchema {
    let mut sections = Vec::new();
    let mut general_fields = Vec::new();

    for node in &ast.roots {
        match &node.kind {
            UiNodeKind::Object { .. } => sections.push(build_section_from_object(node)),
            _ => general_fields.push(field_schema_from_node(node, "general")),
        }
    }

    if !general_fields.is_empty() {
        sections.insert(
            0,
            FormSection {
                id: "general".into(),
                title: "General".into(),
                description: None,
                path: Vec::new(),
                fields: general_fields,
                children: Vec::new(),
            },
        );
    }

    if sections.is_empty() {
        sections.push(FormSection {
            id: "general".into(),
            title: "General".into(),
            description: None,
            path: Vec::new(),
            fields: Vec::new(),
            children: Vec::new(),
        });
    }

    FormSchema {
        title: None,
        description: None,
        roots: vec![RootSection {
            id: "root".into(),
            title: "Schema".into(),
            description: None,
            sections,
        }],
    }
}

fn build_section_from_object(node: &UiNode) -> FormSection {
    let pointer_path = pointer_segments(&node.pointer);
    let id = section_id(&pointer_path);
    let title = node
        .title
        .clone()
        .or_else(|| pointer_path.last().cloned())
        .unwrap_or_else(|| "Section".into());
    let description = node.description.clone();
    let mut fields = Vec::new();
    let mut children = Vec::new();

    if let UiNodeKind::Object {
        children: inner, ..
    } = &node.kind
    {
        for child in inner {
            match &child.kind {
                UiNodeKind::Object { .. } => children.push(build_section_from_object(child)),
                _ => fields.push(field_schema_from_node(child, &id)),
            }
        }
    }

    FormSection {
        id,
        title,
        description,
        path: pointer_path,
        fields,
        children,
    }
}

fn field_schema_from_node(node: &UiNode, section_id: &str) -> FieldSchema {
    let path = pointer_segments(&node.pointer);
    let name = path
        .last()
        .cloned()
        .unwrap_or_else(|| node.pointer.trim_start_matches('/').to_string());
    let title = node.title.clone().unwrap_or_else(|| prettify_label(&name));

    FieldSchema {
        name,
        path,
        pointer: node.pointer.clone(),
        title,
        description: node.description.clone(),
        section_id: section_id.to_string(),
        kind: field_kind_from_node_kind(&node.kind),
        required: node.required,
        default: node.default_value.clone(),
        metadata: HashMap::new(),
    }
}

fn field_kind_from_node_kind(kind: &UiNodeKind) -> FieldKind {
    match kind {
        UiNodeKind::Field {
            scalar,
            enum_options,
        } => match enum_options {
            Some(options) if !options.is_empty() => FieldKind::Enum(options.clone()),
            _ => match scalar {
                ScalarKind::String => FieldKind::String,
                ScalarKind::Integer => FieldKind::Integer,
                ScalarKind::Number => FieldKind::Number,
                ScalarKind::Boolean => FieldKind::Boolean,
            },
        },
        UiNodeKind::Array { item, .. } => {
            FieldKind::Array(Box::new(field_kind_from_node_kind(item)))
        }
        UiNodeKind::Composite { mode, variants, .. } => {
            FieldKind::Composite(Box::new(CompositeField {
                mode: match mode {
                    CompositeMode::OneOf => DomainCompositeMode::OneOf,
                    CompositeMode::AnyOf => DomainCompositeMode::AnyOf,
                },
                variants: variants.iter().map(to_composite_variant).collect(),
            }))
        }
        UiNodeKind::Object { .. } => FieldKind::Json,
    }
}

fn to_composite_variant(variant: &UiVariant) -> CompositeVariant {
    CompositeVariant {
        id: variant.id.clone(),
        title: variant.title.clone().unwrap_or_else(|| variant.id.clone()),
        description: variant.description.clone(),
        schema: variant.schema.clone(),
        is_object: variant.is_object,
    }
}

fn pointer_segments(pointer: &str) -> Vec<String> {
    if pointer.is_empty() || pointer == "/" {
        return Vec::new();
    }
    pointer
        .split('/')
        .skip(1)
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .collect()
}

fn section_id(path: &[String]) -> String {
    if path.is_empty() {
        "root".into()
    } else {
        path.join("-")
    }
}

fn prettify_label(input: &str) -> String {
    let mut chars = input.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if ch == '_' || ch == '-' {
            out.push(' ');
            continue;
        }
        if out.is_empty() {
            out.push(ch.to_ascii_uppercase());
        } else if ch.is_uppercase()
            && chars
                .peek()
                .map(|next| next.is_lowercase())
                .unwrap_or(false)
        {
            out.push(' ');
            out.push(ch);
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        input.to_string()
    } else {
        out
    }
}
