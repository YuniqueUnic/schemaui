use std::collections::BTreeSet;

use super::{UiAst, UiNode, UiNodeKind};

/// Collect all JSON Pointer strings that appear in a UiAst tree.
///
/// This helper is pure and makes no assumptions about how the AST will be
/// rendered. It is primarily useful for tests, diagnostics, or building
/// pointer indexes in higher layers.
pub fn collect_pointers(ast: &UiAst) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for node in &ast.roots {
        collect_node_pointers(node, &mut set);
    }
    set
}

fn collect_node_pointers(node: &UiNode, out: &mut BTreeSet<String>) {
    out.insert(node.pointer.clone());
    match &node.kind {
        UiNodeKind::Object { children, .. } => {
            for child in children {
                collect_node_pointers(child, out);
            }
        }
        UiNodeKind::Composite { variants, .. } => {
            for variant in variants {
                collect_kind_pointers(&variant.node, out);
            }
        }
        UiNodeKind::Array { item, .. } => {
            collect_kind_pointers(item, out);
        }
        UiNodeKind::Field { .. } => {}
    }
}

fn collect_kind_pointers(kind: &UiNodeKind, out: &mut BTreeSet<String>) {
    match kind {
        UiNodeKind::Object { children, .. } => {
            for child in children {
                collect_node_pointers(child, out);
            }
        }
        UiNodeKind::Composite { variants, .. } => {
            for variant in variants {
                collect_kind_pointers(&variant.node, out);
            }
        }
        UiNodeKind::Array { item, .. } => {
            collect_kind_pointers(item, out);
        }
        UiNodeKind::Field { .. } => {}
    }
}
