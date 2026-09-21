//! WebAssembly bindings for the schemaui core pipeline.
//!
//! These five exports mirror the HTTP contract exactly:
//!
//! | wasm export        | HTTP equivalent          |
//! |--------------------|--------------------------|
//! | `buildUiAst`       | `GET /api/v1/session`    |
//! | `validate`         | `POST /api/v1/validate`  |
//! | `render`           | `POST /api/v1/preview`   |
//! | `schemaWithDefaults` | the merge `buildUiAst` applies internally, exposed standalone |
//! | `schemaFromData`   | schema inference, as the CLI does for a schema-less config file |
//!
//! **Error handling.** Every function returns a `Result` that becomes a JS
//! `Error` on failure. The caller checks with `try { … } catch (e) { … }`.
//! There is no intermediate error type — the message string is the contract.
//!
//! **Serialization.** All JSON goes through `serde_json::Value` → JS via
//! `serde-wasm-bindgen`, which avoids double-encoding and keeps the types
//! natural on the JS side.

use wasm_bindgen::prelude::*;

// Set up better panic messages in the browser console in debug builds.
#[cfg(feature = "console_error_panic_hook")]
pub use console_error_panic_hook::set_once as set_panic_hook;

/// Build a `UiAst` from a JSON Schema value and optional defaults.
///
/// `schema` — the JSON Schema as a JS value.
/// `defaults` — optional initial data (`null` or `undefined` → empty object).
///
/// Returns `{ ui_ast, layout, data, formats }` — the same shape as
/// `GET /api/v1/session`, minus the server-side metadata (`title`,
/// `description`, `api_version`, `capabilities`, `expires_in_ms`).
///
/// # Errors
/// Propagates schema-parse and UiAst-build errors as JS `Error`s.
#[wasm_bindgen(js_name = buildUiAst)]
pub fn build_ui_ast(schema: JsValue, defaults: JsValue) -> Result<JsValue, JsValue> {
    let schema: serde_json::Value = serde_wasm_bindgen::from_value(schema)
        .map_err(|e| JsValue::from_str(&format!("invalid schema: {e}")))?;

    let defaults: serde_json::Value = if defaults.is_null() || defaults.is_undefined() {
        serde_json::Value::Object(Default::default())
    } else {
        serde_wasm_bindgen::from_value(defaults)
            .map_err(|e| JsValue::from_str(&format!("invalid defaults: {e}")))?
    };

    let result = schemaui::wasm_core::build_ui_ast(schema, defaults)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate `data` against `schema`.
///
/// Returns `{ ok: boolean, errors: Array<{ pointer: string, message: string }> }`.
/// This matches `POST /api/v1/validate` exactly, field names included.
///
/// # Errors
/// Returns a JS `Error` when the schema itself is invalid.
#[wasm_bindgen]
pub fn validate(schema: JsValue, data: JsValue) -> Result<JsValue, JsValue> {
    let schema: serde_json::Value = serde_wasm_bindgen::from_value(schema)
        .map_err(|e| JsValue::from_str(&format!("invalid schema: {e}")))?;
    let data: serde_json::Value = serde_wasm_bindgen::from_value(data)
        .map_err(|e| JsValue::from_str(&format!("invalid data: {e}")))?;

    let result = schemaui::wasm_core::validate(schema, data)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Render `data` to a document string in the requested format.
///
/// `format` — `"json"`, `"yaml"`, or `"toml"` (availability depends on features).
/// `pretty` — whether to pretty-print the output.
///
/// Returns `{ payload: string }`. Matches `POST /api/v1/preview`.
///
/// # Errors
/// Returns a JS `Error` when serialization fails (e.g. TOML with null values).
#[wasm_bindgen]
pub fn render(data: JsValue, format: &str, pretty: bool) -> Result<JsValue, JsValue> {
    let data: serde_json::Value = serde_wasm_bindgen::from_value(data)
        .map_err(|e| JsValue::from_str(&format!("invalid data: {e}")))?;

    let result = schemaui::wasm_core::render(data, format, pretty)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Merge schema `default` values into `data`.
///
/// Fields present in `schema.properties[k].default` that are absent in `data`
/// are filled in. This is what the session pipeline does before handing data to
/// the frontend, so both transports start from the same baseline.
///
/// Returns the merged data object.
///
/// # Errors
/// Returns a JS `Error` when deserialization fails.
#[wasm_bindgen(js_name = schemaWithDefaults)]
pub fn schema_with_defaults(schema: JsValue, data: JsValue) -> Result<JsValue, JsValue> {
    let schema: serde_json::Value = serde_wasm_bindgen::from_value(schema)
        .map_err(|e| JsValue::from_str(&format!("invalid schema: {e}")))?;
    let data: serde_json::Value = serde_wasm_bindgen::from_value(data)
        .map_err(|e| JsValue::from_str(&format!("invalid data: {e}")))?;

    let result = schemaui::wasm_core::schema_with_defaults(schema, data);
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Infer a JSON Schema from a data value, the way the CLI does when no
/// schema is given for a config file.
///
/// Returns a JSON Schema whose `properties` and `default`s mirror `data`'s
/// shape — a starting point for editing, not a hand-authored schema.
///
/// # Errors
/// Returns a JS `Error` when deserialization fails.
#[wasm_bindgen(js_name = schemaFromData)]
pub fn schema_from_data(data: JsValue) -> Result<JsValue, JsValue> {
    let data: serde_json::Value = serde_wasm_bindgen::from_value(data)
        .map_err(|e| JsValue::from_str(&format!("invalid data: {e}")))?;

    let result = schemaui::wasm_core::schema_from_data(data);
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}
