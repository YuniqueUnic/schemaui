use schemars::schema::SchemaObject;

use crate::schema::metadata::section_info_for_object;
use crate::tui::model::form_schema::{FormSection, RootSection};

#[derive(Debug, Clone)]
pub(super) struct RootBuilder {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) description: Option<String>,
    pub(super) sections: Vec<FormSection>,
}

impl RootBuilder {
    pub(super) fn new(name: &str, schema: &SchemaObject) -> Self {
        let meta = section_info_for_object(schema, name, None);
        Self {
            id: name.to_string(),
            title: meta.title,
            description: meta.description,
            sections: Vec::new(),
        }
    }

    pub(super) fn into_root(self) -> RootSection {
        RootSection {
            id: self.id,
            title: self.title,
            description: self.description,
            sections: self.sections,
        }
    }
}
