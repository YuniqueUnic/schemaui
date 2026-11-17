use serde::{Deserialize, Serialize};

use crate::ui_ast::{
    EnumDetail, FieldBounds, FieldControl, RichContent, UiNode, UiNodeKind, VisibleWhen,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NodePresentation {
    pub control: Option<FieldControl>,
    pub bounds: Option<FieldBounds>,
    pub content: Option<RichContent>,
    pub visible_when: Option<VisibleWhen>,
    pub multiline: bool,
    pub enum_details: Vec<EnumDetail>,
}

impl NodePresentation {
    pub fn is_multiline(&self) -> bool {
        match self.control {
            Some(FieldControl::Textarea | FieldControl::Mermaid) => true,
            Some(FieldControl::Text) => false,
            _ => self.multiline,
        }
    }
}

impl From<&UiNode> for NodePresentation {
    fn from(node: &UiNode) -> Self {
        Self {
            control: node.control,
            bounds: node.bounds.clone(),
            content: node.content.clone(),
            visible_when: node.visible_when.clone(),
            ..Self::from(&node.kind)
        }
    }
}

impl From<&UiNodeKind> for NodePresentation {
    fn from(kind: &UiNodeKind) -> Self {
        let field = match kind {
            UiNodeKind::Array { item, .. } => item.as_ref(),
            kind => kind,
        };
        let (multiline, enum_details) = match field {
            UiNodeKind::Field {
                multiline,
                enum_details,
                ..
            } => (*multiline, enum_details.clone().unwrap_or_default()),
            _ => (false, Vec::new()),
        };
        Self {
            multiline,
            enum_details,
            ..Self::default()
        }
    }
}
