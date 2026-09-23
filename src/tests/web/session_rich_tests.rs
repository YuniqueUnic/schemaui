//! The session contract carries rich assets.

use serde_json::json;

use crate::web::session::WebSessionBuilder;

const FIGURE: &str = r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/></svg>"#;

fn schema_with(figure: bool) -> serde_json::Value {
    // The property is always present; only the `x-content` key comes and
    // goes, because an explicit `null` would be a malformed declaration
    // rather than an absent one.
    let mut field = json!({ "type": "string" });
    if figure {
        field.as_object_mut().expect("object").insert(
            "x-content".to_string(),
            json!({ "type": "svg", "source": FIGURE }),
        );
    }
    json!({
        "type": "object",
        "properties": { "icon": field }
    })
}

#[test]
fn a_session_with_declared_content_ships_its_assets() {
    let config = WebSessionBuilder::new(schema_with(true))
        .build()
        .expect("session builds");
    let response = config.session_response();

    if cfg!(feature = "rich") {
        let assets = response.rich.expect("svg content renders in a rich build");
        assert_eq!(assets.len(), 1);
    } else {
        // Nothing this build can render: the field stays, the map does not,
        // and the frontend falls back to the source it still holds.
        assert_eq!(response.rich, None);
    }
}

#[test]
fn a_session_without_rich_content_has_no_map_at_all() {
    let config = WebSessionBuilder::new(schema_with(false))
        .build()
        .expect("session builds");
    assert_eq!(config.session_response().rich, None);
}
