//! WebAssembly bindings for the schemaui core pipeline.
//!
//! Five of these exports mirror the HTTP contract exactly:
//!
//! | wasm export        | HTTP equivalent          |
//! |--------------------|--------------------------|
//! | `buildUiAst`       | `GET /api/v1/session`    |
//! | `validate`         | `POST /api/v1/validate`  |
//! | `render`           | `POST /api/v1/preview`   |
//! | `schemaWithDefaults` | the merge `buildUiAst` applies internally, exposed standalone |
//! | `schemaFromData`   | schema inference, as the CLI does for a schema-less config file |
//!
//! A sixth, `parseDocument`, has no HTTP equivalent: a live session only ever
//! receives already-parsed JSON, but the Playground's paste-a-schema screen
//! has no server to have parsed its input, so it needs to accept JSON, YAML
//! or TOML text directly — the same formats `schemaui-cli` accepts from a
//! file.
//!
//! **Error handling.** Every function returns a `Result` that becomes a JS
//! `Error` on failure. The caller checks with `try { … } catch (e) { … }`.
//! There is no intermediate error type — the message string is the contract.
//!
//! **Serialization.** All JSON goes through `serde_json::Value` → JS via
//! `serde-wasm-bindgen`, which avoids double-encoding and keeps the types
//! natural on the JS side. Every return value goes through [`to_js`] rather
//! than `serde_wasm_bindgen::to_value` directly: the plain `to_value` path
//! serializes a JSON object as an ES2015 `Map` (serde-wasm-bindgen's default,
//! optimized for round-tripping Rust maps, not for JSON interop), which reads
//! back on the JS side as an object with no own properties — every `.field`
//! access silently returns `undefined` instead of throwing, so this is a
//! correctness bug that source line counting cannot see, only a JS-level test
//! or a real browser can.

use serde::Serialize;
use wasm_bindgen::prelude::*;

// Set up better panic messages in the browser console in debug builds.
#[cfg(feature = "console_error_panic_hook")]
pub use console_error_panic_hook::set_once as set_panic_hook;

/// Serialize `value` the way JSON.parse would produce it: plain objects and
/// arrays, not ES2015 `Map`s. See the module doc for why this matters.
fn to_js(value: &impl Serialize) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    value
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Build a `UiAst` from a JSON Schema value and optional defaults.
///
/// `schema` — the JSON Schema as a JS value.
/// `defaults` — optional initial data (`null` or `undefined` → empty object).
///
/// Returns `{ ui_ast, layout, rich, data, formats }` — the same shape as
/// `GET /api/v1/session`, minus the server-side metadata (`title`,
/// `description`, `api_version`, `capabilities`, `expires_in_ms`).
///
/// `rich` carries the rendered figures for everything the schema declared;
/// it is absent when there is nothing to render or this build cannot render
/// the declared kinds.
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

    to_js(&result)
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

    to_js(&result)
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

    to_js(&result)
}

/// Render one Mermaid diagram on demand, as SVG in the requested theme.
///
/// `kind` — `"mermaid"` (the only renderable kind: SVG and markdown are
/// session-authored, never user-typed).
/// `theme` — `"light"` or `"dark"`.
///
/// Returns `{ svg: { body, width, height } }`. Matches `POST /api/v1/render`.
/// Only exists in packages built with the `mermaid` feature.
///
/// # Errors
/// Returns a JS `Error` with the renderer's message (including its line and
/// column hint) when the source does not parse.
#[cfg(feature = "mermaid")]
#[wasm_bindgen(js_name = renderRich)]
pub fn render_rich(kind: &str, source: &str, theme: &str) -> Result<JsValue, JsValue> {
    let result = schemaui::wasm_core::render_rich(kind, source, theme)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&result)
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
    to_js(&result)
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
    to_js(&result)
}

/// Parse `text` as JSON, YAML or TOML — whichever it turns out to be.
///
/// For the Playground's paste-a-schema screen: pasted text has no
/// `Content-Type` to say which format it is in, so every enabled format is
/// tried in turn (JSON first) rather than asking the visitor to pick one.
///
/// # Errors
/// Returns a JS `Error` when `text` does not parse as any enabled format.
#[wasm_bindgen(js_name = parseDocument)]
pub fn parse_document(text: &str) -> Result<JsValue, JsValue> {
    let result =
        schemaui::wasm_core::parse_document(text).map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&result)
}
