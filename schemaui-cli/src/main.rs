#![doc = include_str!("../../docs/en/cli_usage.md")]

use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::{ArgAction, Parser};
use color_eyre::eyre::{Report, Result, WrapErr, eyre};
use serde_json::Value;

use schemaui::{
    DocumentFormat, OutputDestination, OutputOptions, SchemaUI, parse_document_str,
    schema_from_data_value, schema_with_defaults,
};

const DEFAULT_TEMP_FILE: &str = "/tmp/schemaui.json";

#[derive(Debug, Parser)]
#[command(
    name = "schemaui",
    version,
    about = "Render JSON Schemas as interactive TUIs"
)]
struct Cli {
    /// Schema spec: file path, inline payload, or "-" for stdin
    #[arg(short = 's', long = "schema", value_name = "SPEC")]
    schema: Option<String>,

    /// Config spec: file path, inline payload, or "-" for stdin
    #[arg(short = 'c', long = "config", alias = "data", value_name = "SPEC")]
    config: Option<String>,

    /// Title shown at the top of the UI
    #[arg(long = "title", value_name = "TEXT")]
    title: Option<String>,

    /// Output destinations ("-" writes to stdout). Accepts multiple values per flag use.
    #[arg(short = 'o', long = "output", value_name = "DEST", num_args = 1.., action = ArgAction::Append)]
    outputs: Vec<String>,

    /// Override the default temp file location (only used when no other destinations are set)
    #[arg(long = "temp-file", value_name = "PATH")]
    temp_file: Option<PathBuf>,

    /// Disable writing to the default temp file when no destinations are provided
    #[arg(long = "no-temp-file")]
    no_temp_file: bool,

    /// Emit compact JSON/TOML rather than pretty formatting
    #[arg(long = "no-pretty")]
    no_pretty: bool,

    /// Overwrite output files even if they already exist
    #[arg(short = 'f', long = "force", short_alias = 'y', alias = "yes")]
    force: bool,
}

#[derive(Debug)]
enum InputSource {
    File(PathBuf),
    Stdin,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let mut diagnostics = DiagnosticCollector::default();

    let schema_spec = cli.schema.as_deref();
    let config_spec = cli.config.as_deref();
    let schema_stdin = schema_spec == Some("-");
    let config_stdin = config_spec == Some("-");
    if schema_stdin && config_stdin {
        diagnostics.push_input(
            "schema/config",
            "cannot read schema and config from stdin simultaneously; provide inline content or files",
        );
    }

    let schema_hint = resolve_format_hint(schema_spec, "schema", &mut diagnostics);
    let config_hint = resolve_format_hint(config_spec, "config", &mut diagnostics);

    let schema_value = load_optional_value(
        schema_spec,
        schema_hint.hint.format,
        "schema",
        schema_hint.blocked || (schema_stdin && config_stdin),
        &mut diagnostics,
    );
    let config_value = load_optional_value(
        config_spec,
        config_hint.hint.format,
        "config",
        config_hint.blocked || (schema_stdin && config_stdin),
        &mut diagnostics,
    );

    let (output_settings, output_paths) = build_output_options(
        &cli,
        config_hint.hint.extension_value(),
        schema_hint.hint.extension_value(),
        &mut diagnostics,
    );
    ensure_output_paths_available(&output_paths, cli.force, &mut diagnostics);

    diagnostics.into_result()?;

    let mut schema_value = schema_value;
    let mut config_value = config_value;

    if schema_value.is_none() {
        if let Some(config_doc) = config_value.as_ref()
            && looks_like_json_schema(config_doc)
        {
            eprintln!(
                "detected JSON Schema provided via --config; treating it as the active schema"
            );
            schema_value = config_value.take();
        }
    }

    if schema_value.is_none() && config_value.is_none() {
        return Err(eyre!("provide at least --schema or --config"));
    }

    let schema = match (schema_value, config_value.as_ref()) {
        (Some(schema), Some(defaults)) => schema_with_defaults(&schema, defaults),
        (Some(schema), None) => schema,
        (None, Some(defaults)) => schema_from_data_value(defaults),
        (None, None) => unreachable!("validated above"),
    };

    let mut ui = SchemaUI::new(schema);
    if let Some(title) = cli.title.as_ref() {
        ui = ui.with_title(title.clone());
    }
    if let Some(defaults) = config_value.as_ref() {
        ui = ui.with_default_data(defaults);
    }

    if let Some(options) = output_settings {
        ui = ui.with_output(options);
    }

    let _ = ui.run().map_err(Report::msg)?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct FormatHint {
    format: DocumentFormat,
    from_extension: bool,
}

impl Default for FormatHint {
    fn default() -> Self {
        Self {
            format: DocumentFormat::default(),
            from_extension: false,
        }
    }
}

impl FormatHint {
    fn extension_value(&self) -> Option<DocumentFormat> {
        self.from_extension.then_some(self.format)
    }
}

#[derive(Debug, Clone, Copy)]
struct FormatResolution {
    hint: FormatHint,
    blocked: bool,
}

fn resolve_format_hint(
    path_hint: Option<&str>,
    label: &str,
    diagnostics: &mut DiagnosticCollector,
) -> FormatResolution {
    if let Some(path) = path_hint {
        if path != "-" {
            match probe_format_from_extension(Path::new(path)) {
                ExtensionFormat::Known(format) => {
                    return FormatResolution {
                        hint: FormatHint {
                            format,
                            from_extension: true,
                        },
                        blocked: false,
                    };
                }
                ExtensionFormat::UnsupportedFeature {
                    format_name,
                    feature_flag,
                } => {
                    diagnostics.push_input(
                        label,
                        format!(
                            "{label} '{path}' requires {format_name} support, but this build lacks the '{feature_flag}' feature"
                        ),
                    );
                    return FormatResolution {
                        hint: FormatHint::default(),
                        blocked: true,
                    };
                }
                ExtensionFormat::Unknown => {}
            }
        }
    }

    FormatResolution {
        hint: FormatHint::default(),
        blocked: false,
    }
}

fn load_optional_value(
    spec: Option<&str>,
    format: DocumentFormat,
    label: &str,
    skip: bool,
    diagnostics: &mut DiagnosticCollector,
) -> Option<Value> {
    if skip {
        return None;
    }
    let Some(raw) = spec else {
        return None;
    };
    match load_value(raw, format, label) {
        Ok(value) => Some(value),
        Err(err) => {
            diagnostics.push_input(label, err.to_string());
            None
        }
    }
}

fn load_value(spec: &str, format: DocumentFormat, label: &str) -> Result<Value> {
    if spec == "-" {
        let contents = read_from_source(&InputSource::Stdin)?;
        return parse_contents(&contents, format, label);
    }

    let path = PathBuf::from(spec);
    match read_from_source(&InputSource::File(path.clone())) {
        Ok(contents) => parse_contents(&contents, format, label),
        Err(err) => {
            if is_not_found(&err) {
                let inline_label = format!("inline {label}");
                return parse_contents(spec, format, &inline_label);
            }
            Err(err.wrap_err(format!("failed to load {label} from {}", path.display())))
        }
    }
}

fn read_from_source(source: &InputSource) -> Result<String> {
    match source {
        InputSource::Stdin => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .wrap_err("failed to read from stdin")?;
            Ok(buffer)
        }
        InputSource::File(path) => fs::read_to_string(path)
            .wrap_err_with(|| format!("failed to read file {}", path.display())),
    }
}

fn is_not_found(err: &Report) -> bool {
    err.downcast_ref::<io::Error>()
        .map_or(false, |io_err| io_err.kind() == io::ErrorKind::NotFound)
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
            Err(Report::msg(format!(
                "failed to parse {label}: tried {} (first error: {primary})",
                format_list()
            )))
        }
    }
}

fn looks_like_json_schema(value: &Value) -> bool {
    let obj = match value.as_object() {
        Some(map) => map,
        None => return false,
    };

    if obj
        .get("properties")
        .and_then(Value::as_object)
        .map(|props| props.len())
        .unwrap_or(0)
        == 0
    {
        return false;
    }

    if obj.contains_key("$schema") {
        return true;
    }

    if matches!(obj.get("type"), Some(Value::String(t)) if t == "object") {
        return true;
    }

    if let Some(props) = obj.get("properties").and_then(Value::as_object) {
        let mut scored = 0usize;
        for value in props.values() {
            if let Some(prop_obj) = value.as_object() {
                if prop_obj.contains_key("type")
                    || prop_obj.contains_key("properties")
                    || prop_obj.contains_key("items")
                    || prop_obj.contains_key("$ref")
                {
                    scored += 1;
                }
            }
        }
        if scored > 0 {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::looks_like_json_schema;
    use serde_json::json;

    #[test]
    fn detects_json_schema_shape() {
        let doc = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "username": {"type": "string"},
                "tags": {"type": "array", "items": {"type": "string"}}
            }
        });
        assert!(looks_like_json_schema(&doc));
    }

    #[test]
    fn ignores_regular_config_documents() {
        let doc = json!({
            "username": "unic",
            "tags": ["alpha"],
            "properties": "not a schema"
        });
        assert!(!looks_like_json_schema(&doc));
    }
}

fn format_list() -> String {
    let items: Vec<String> = DocumentFormat::available_formats()
        .into_iter()
        .map(|fmt| fmt.to_string())
        .collect();
    items.join(", ")
}

#[derive(Default)]
struct DiagnosticCollector {
    messages: Vec<String>,
}

impl DiagnosticCollector {
    fn push_input(&mut self, label: &str, message: impl Into<String>) {
        self.messages
            .push(format!("input ({label}): {}", message.into()));
    }

    fn push_output(&mut self, message: impl Into<String>) {
        self.messages.push(format!("output: {}", message.into()));
    }

    fn len(&self) -> usize {
        self.messages.len()
    }

    fn into_result(self) -> Result<()> {
        if self.messages.is_empty() {
            return Ok(());
        }
        let mut body = String::from("encountered input/output issues:\n");
        for (idx, msg) in self.messages.iter().enumerate() {
            let _ = writeln!(body, "  {}. {}", idx + 1, msg);
        }
        Err(eyre!(body))
    }
}

fn build_output_options(
    cli: &Cli,
    config_hint: Option<DocumentFormat>,
    schema_hint: Option<DocumentFormat>,
    diagnostics: &mut DiagnosticCollector,
) -> (Option<OutputOptions>, Vec<PathBuf>) {
    let mut destinations = Vec::new();
    let explicit_outputs = !cli.outputs.is_empty();

    for raw in &cli.outputs {
        if raw.trim().is_empty() {
            diagnostics.push_output("output destination cannot be empty");
            continue;
        }
        if raw == "-" {
            destinations.push(OutputDestination::Stdout);
        } else {
            destinations.push(OutputDestination::file(raw));
        }
    }

    if destinations.is_empty() && !explicit_outputs {
        if cli.no_temp_file {
            return (None, Vec::new());
        }
        let fallback = cli
            .temp_file
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_TEMP_FILE));
        destinations.push(OutputDestination::file(fallback.clone()));
    }

    if destinations.is_empty() {
        return (None, Vec::new());
    }

    let file_paths: Vec<PathBuf> = destinations
        .iter()
        .filter_map(|dest| match dest {
            OutputDestination::File(path) => Some(path.clone()),
            OutputDestination::Stdout => None,
        })
        .collect();

    let start = diagnostics.len();
    let format = if file_paths.is_empty() {
        determine_stdout_format(config_hint, schema_hint)
    } else {
        infer_format_from_files(&file_paths, diagnostics).unwrap_or_else(DocumentFormat::default)
    };

    if diagnostics.len() > start {
        return (None, file_paths);
    }

    (
        Some(OutputOptions {
            format,
            pretty: !cli.no_pretty,
            destinations,
        }),
        file_paths,
    )
}

fn determine_stdout_format(
    config_hint: Option<DocumentFormat>,
    schema_hint: Option<DocumentFormat>,
) -> DocumentFormat {
    config_hint
        .or(schema_hint)
        .unwrap_or_else(DocumentFormat::default)
}

fn infer_format_from_files(
    file_paths: &[PathBuf],
    diagnostics: &mut DiagnosticCollector,
) -> Option<DocumentFormat> {
    let mut detected: Option<DocumentFormat> = None;
    for path in file_paths {
        match probe_format_from_extension(path) {
            ExtensionFormat::Known(format) => {
                if let Some(existing) = detected {
                    if existing != format {
                        diagnostics.push_output(format!(
                            "output file {} uses {format} but other destinations use {existing}; align extensions",
                            path.display()
                        ));
                    }
                } else {
                    detected = Some(format);
                }
            }
            ExtensionFormat::UnsupportedFeature {
                format_name,
                feature_flag,
            } => diagnostics.push_output(format!(
                "output file {} requires {format_name} support, but this build was compiled without the '{feature_flag}' feature",
                path.display()
            )),
            ExtensionFormat::Unknown => diagnostics.push_output(format!(
                "cannot infer format from output file {}; use .json/.yaml/.toml",
                path.display()
            )),
        }
    }
    detected
}

fn probe_format_from_extension(path: &Path) -> ExtensionFormat {
    let Some(ext) = path.extension() else {
        return ExtensionFormat::Unknown;
    };
    let normalized = ext.to_string_lossy().to_ascii_lowercase();
    match normalized.as_str() {
        "json" => ExtensionFormat::Known(DocumentFormat::Json),
        #[cfg(feature = "yaml")]
        "yaml" | "yml" => ExtensionFormat::Known(DocumentFormat::Yaml),
        #[cfg(not(feature = "yaml"))]
        "yaml" | "yml" => ExtensionFormat::UnsupportedFeature {
            format_name: "yaml",
            feature_flag: "yaml",
        },
        #[cfg(feature = "toml")]
        "toml" => ExtensionFormat::Known(DocumentFormat::Toml),
        #[cfg(not(feature = "toml"))]
        "toml" => ExtensionFormat::UnsupportedFeature {
            format_name: "toml",
            feature_flag: "toml",
        },
        _ => ExtensionFormat::Unknown,
    }
}

#[derive(Debug)]
enum ExtensionFormat {
    Known(DocumentFormat),
    #[allow(dead_code)]
    UnsupportedFeature {
        format_name: &'static str,
        feature_flag: &'static str,
    },
    Unknown,
}

fn ensure_output_paths_available(
    paths: &[PathBuf],
    force: bool,
    diagnostics: &mut DiagnosticCollector,
) {
    if force {
        return;
    }
    for path in paths {
        if path.exists() {
            diagnostics.push_output(format!(
                "file {} already exists (pass --force to overwrite)",
                path.display()
            ));
        }
    }
}
