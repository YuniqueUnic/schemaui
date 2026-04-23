use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
#[cfg(feature = "remote-schema")]
use std::time::Duration;

use color_eyre::eyre::{Report, Result, WrapErr, eyre};
#[cfg(feature = "remote-schema")]
use reqwest::blocking::Client;
use schemaui::{
    DocumentFormat, DocumentFormatProbe, looks_like_json_schema, parse_document_str,
    schema_from_data_value,
};
use serde_json::Value;
use url::Url;

use super::diagnostics::DiagnosticCollector;

#[derive(Debug, Clone)]
pub(crate) enum DocumentOrigin {
    File(PathBuf),
    #[cfg(feature = "remote-schema")]
    Url(Url),
    Inline,
    Stdin,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedDocument {
    pub value: Value,
    #[cfg(any(feature = "yaml", feature = "toml"))]
    pub raw: String,
    pub format: DocumentFormat,
    pub origin: DocumentOrigin,
}

#[derive(Debug)]
pub(crate) struct ResolvedSessionInputs {
    pub schema: Value,
    pub defaults: Option<Value>,
}

pub(crate) fn load_optional_document(
    spec: Option<&str>,
    format: DocumentFormat,
    label: &str,
    skip: bool,
    diagnostics: &mut DiagnosticCollector,
) -> Option<LoadedDocument> {
    if skip {
        return None;
    }
    let spec = spec?;
    match load_document(spec, format, label) {
        Ok(document) => Some(document),
        Err(err) => {
            diagnostics.push_input(label, err.to_string());
            None
        }
    }
}

pub(crate) fn load_document(
    spec: &str,
    format: DocumentFormat,
    label: &str,
) -> Result<LoadedDocument> {
    if spec == "-" {
        let raw = read_stdin()?;
        let value = parse_contents(&raw, format, label)?;
        return Ok(LoadedDocument {
            value,
            #[cfg(any(feature = "yaml", feature = "toml"))]
            raw,
            format,
            origin: DocumentOrigin::Stdin,
        });
    }

    if let Some(url) = parse_special_url(spec) {
        return load_document_from_url(url, format, label);
    }

    let path = PathBuf::from(spec);
    match fs::read_to_string(&path) {
        Ok(raw) => {
            let value = parse_contents(&raw, format, label)?;
            Ok(LoadedDocument {
                value,
                #[cfg(any(feature = "yaml", feature = "toml"))]
                raw,
                format,
                origin: DocumentOrigin::File(path),
            })
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            let inline_label = format!("inline {label}");
            let value = parse_contents(spec, format, &inline_label)?;
            Ok(LoadedDocument {
                value,
                #[cfg(any(feature = "yaml", feature = "toml"))]
                raw: spec.to_string(),
                format,
                origin: DocumentOrigin::Inline,
            })
        }
        Err(err) => {
            Err(Report::new(err)
                .wrap_err(format!("failed to load {label} from {}", path.display())))
        }
    }
}

pub(crate) fn resolve_session_inputs(
    explicit_schema: Option<LoadedDocument>,
    config: Option<LoadedDocument>,
) -> Result<ResolvedSessionInputs> {
    let explicit_schema = explicit_schema.map(|document| document.value);

    if explicit_schema.is_none()
        && let Some(config) = config.as_ref()
        && looks_like_json_schema(&config.value)
    {
        eprintln!("detected JSON Schema provided via --config; treating it as the active schema");
        return Ok(ResolvedSessionInputs {
            schema: config.value.clone(),
            defaults: None,
        });
    }

    let config = config.map(PreparedConfig::from_loaded).transpose()?;

    let schema = if let Some(schema) = explicit_schema {
        schema
    } else if let Some(reference) = config
        .as_ref()
        .and_then(|config| config.declared_schema.as_deref())
    {
        load_schema_reference(reference, config.as_ref().map(|config| &config.origin))?
    } else if let Some(config) = config.as_ref() {
        schema_from_data_value(&config.value)
    } else {
        return Err(eyre!("provide at least --schema or --config"));
    };

    Ok(ResolvedSessionInputs {
        schema,
        defaults: config.map(|config| config.value),
    })
}

#[derive(Debug)]
struct PreparedConfig {
    value: Value,
    declared_schema: Option<String>,
    origin: DocumentOrigin,
}

impl PreparedConfig {
    fn from_loaded(document: LoadedDocument) -> Result<Self> {
        let declared_schema = detect_declared_schema(&document);
        let value = match document.format {
            #[cfg(feature = "json")]
            DocumentFormat::Json if declared_schema.is_some() => {
                strip_root_json_schema_declaration(document.value)
            }
            _ => document.value,
        };

        Ok(Self {
            value,
            declared_schema,
            origin: document.origin,
        })
    }
}

fn load_schema_reference(reference: &str, base: Option<&DocumentOrigin>) -> Result<Value> {
    let location = resolve_schema_reference(reference, base)?;
    let format = format_for_reference_location(&location)?;
    match location {
        ReferenceLocation::File(path) => {
            let raw = fs::read_to_string(&path)
                .wrap_err_with(|| format!("failed to read schema file {}", path.display()))?;
            parse_contents(&raw, format, "schema")
        }
        ReferenceLocation::Url(url) => {
            let document = load_document_from_url(url, format, "schema")?;
            Ok(document.value)
        }
    }
}

fn resolve_schema_reference(
    reference: &str,
    base: Option<&DocumentOrigin>,
) -> Result<ReferenceLocation> {
    if let Some(url) = parse_special_url(reference) {
        return match url.scheme() {
            "file" => {
                let path = url
                    .to_file_path()
                    .map_err(|_| eyre!("invalid file:// schema reference: {url}"))?;
                Ok(ReferenceLocation::File(path))
            }
            _ => Ok(ReferenceLocation::Url(url)),
        };
    }

    let reference_path = PathBuf::from(reference);
    if reference_path.is_absolute() {
        return Ok(ReferenceLocation::File(reference_path));
    }

    match base {
        Some(DocumentOrigin::File(path)) => {
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            Ok(ReferenceLocation::File(parent.join(reference_path)))
        }
        #[cfg(feature = "remote-schema")]
        Some(DocumentOrigin::Url(url)) => {
            let joined = url
                .join(reference)
                .wrap_err_with(|| format!("failed to resolve schema reference '{reference}'"))?;
            if joined.scheme() == "file" {
                let path = joined
                    .to_file_path()
                    .map_err(|_| eyre!("invalid file:// schema reference: {joined}"))?;
                Ok(ReferenceLocation::File(path))
            } else {
                Ok(ReferenceLocation::Url(joined))
            }
        }
        Some(DocumentOrigin::Inline) | Some(DocumentOrigin::Stdin) | None => {
            let cwd = env::current_dir().wrap_err("failed to resolve current working directory")?;
            Ok(ReferenceLocation::File(cwd.join(reference_path)))
        }
    }
}

#[derive(Debug)]
enum ReferenceLocation {
    File(PathBuf),
    Url(Url),
}

fn format_for_reference_location(location: &ReferenceLocation) -> Result<DocumentFormat> {
    let probe = match location {
        ReferenceLocation::File(path) => DocumentFormat::probe_extension(path),
        ReferenceLocation::Url(url) => DocumentFormat::probe_extension(Path::new(url.path())),
    };

    match probe {
        DocumentFormatProbe::Known(format) => Ok(format),
        DocumentFormatProbe::UnsupportedFeature {
            format_name,
            feature_flag,
        } => Err(eyre!(
            "schema reference {} requires {format_name} support, but this build lacks the '{feature_flag}' feature",
            reference_location_label(location)
        )),
        DocumentFormatProbe::Unknown => Ok(DocumentFormat::default()),
    }
}

fn detect_declared_schema(document: &LoadedDocument) -> Option<String> {
    match document.format {
        #[cfg(feature = "json")]
        DocumentFormat::Json => document
            .value
            .as_object()
            .and_then(|object| object.get("$schema"))
            .and_then(Value::as_str)
            .map(str::to_string),
        #[cfg(feature = "yaml")]
        DocumentFormat::Yaml => detect_yaml_schema_directive(&document.raw),
        #[cfg(feature = "toml")]
        DocumentFormat::Toml => detect_toml_schema_directive(&document.raw),
    }
}

#[cfg(feature = "yaml")]
fn detect_yaml_schema_directive(raw: &str) -> Option<String> {
    for (index, line) in raw.lines().enumerate() {
        let trimmed = if index == 0 {
            line.trim_start_matches('\u{feff}').trim()
        } else {
            line.trim()
        };

        if trimmed.is_empty() {
            continue;
        }
        let Some(comment) = trimmed.strip_prefix('#') else {
            break;
        };
        let comment = comment.trim();

        if let Some(rest) = comment.strip_prefix("yaml-language-server:") {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix("$schema") {
                let rest = rest.trim_start();
                let rest = rest.strip_prefix('=')?.trim();
                if !rest.is_empty() {
                    return Some(rest.to_string());
                }
            }
        }

        if let Some(rest) = comment.strip_prefix("@schema") {
            let rest = rest.trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

#[cfg(feature = "toml")]
fn detect_toml_schema_directive(raw: &str) -> Option<String> {
    for (index, line) in raw.lines().enumerate() {
        let trimmed = if index == 0 {
            line.trim_start_matches('\u{feff}').trim()
        } else {
            line.trim()
        };

        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("#:schema") {
            let rest = rest.trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }

        if trimmed.starts_with('#') {
            continue;
        }
        break;
    }
    None
}

fn strip_root_json_schema_declaration(value: Value) -> Value {
    match value {
        Value::Object(mut object) => {
            object.remove("$schema");
            Value::Object(object)
        }
        other => other,
    }
}

fn load_document_from_url(url: Url, format: DocumentFormat, label: &str) -> Result<LoadedDocument> {
    match url.scheme() {
        "file" => {
            let path = url
                .to_file_path()
                .map_err(|_| eyre!("invalid file:// URL for {label}: {url}"))?;
            let raw = fs::read_to_string(&path)
                .wrap_err_with(|| format!("failed to read {label} file {}", path.display()))?;
            let value = parse_contents(&raw, format, label)?;
            Ok(LoadedDocument {
                value,
                #[cfg(any(feature = "yaml", feature = "toml"))]
                raw,
                format,
                origin: DocumentOrigin::File(path),
            })
        }
        #[cfg(feature = "remote-schema")]
        "http" | "https" => {
            let mut client = Client::builder().timeout(Duration::from_secs(15));
            if should_bypass_proxy(&url) {
                client = client.no_proxy();
            }
            let client = client
                .build()
                .wrap_err("failed to initialize HTTP client")?;
            let response = client
                .get(url.clone())
                .send()
                .wrap_err_with(|| format!("failed to fetch {label} from {url}"))?;
            let status = response.status();
            if !status.is_success() {
                return Err(eyre!(
                    "failed to fetch {label} from {url}: HTTP {}",
                    status.as_u16()
                ));
            }
            let raw = response
                .text()
                .wrap_err_with(|| format!("failed to read {label} response body from {url}"))?;
            let value = parse_contents(&raw, format, label)?;
            Ok(LoadedDocument {
                value,
                #[cfg(any(feature = "yaml", feature = "toml"))]
                raw,
                format,
                #[cfg(feature = "remote-schema")]
                origin: DocumentOrigin::Url(url),
                #[cfg(not(feature = "remote-schema"))]
                origin: DocumentOrigin::Inline,
            })
        }
        #[cfg(not(feature = "remote-schema"))]
        "http" | "https" => Err(eyre!(
            "remote schema support is disabled in this build; re-enable the 'remote-schema' feature to load {label} from {url}"
        )),
        _ => Err(eyre!("unsupported URL scheme for {label}: {url}")),
    }
}

fn parse_special_url(spec: &str) -> Option<Url> {
    let url = Url::parse(spec).ok()?;
    match url.scheme() {
        "http" | "https" | "file" => Some(url),
        _ => None,
    }
}

#[cfg(feature = "remote-schema")]
fn should_bypass_proxy(url: &Url) -> bool {
    matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"))
}

fn read_stdin() -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .wrap_err("failed to read from stdin")?;
    Ok(buffer)
}

fn parse_contents(contents: &str, format: DocumentFormat, label: &str) -> Result<Value> {
    match parse_document_str(contents, format) {
        Ok(value) => Ok(value),
        Err(primary) => {
            for candidate in DocumentFormat::available_formats() {
                if candidate == format {
                    continue;
                }
                if let Ok(value) = parse_document_str(contents, candidate) {
                    return Ok(value);
                }
            }
            Err(eyre!(
                "failed to parse {label}: tried {} (first error: {primary})",
                DocumentFormat::format_list()
            ))
        }
    }
}

fn reference_location_label(location: &ReferenceLocation) -> String {
    match location {
        ReferenceLocation::File(path) => path.display().to_string(),
        ReferenceLocation::Url(url) => url.to_string(),
    }
}
