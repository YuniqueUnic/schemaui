use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::Parser;
use color_eyre::eyre::{Report, Result, WrapErr, eyre};
use serde_json::Value;

use schemaui::{
    DocumentFormat, OutputDestination, OutputOptions, SchemaUI, parse_document_str,
    schema_from_data_value, schema_with_defaults,
};

const DEFAULT_TEMP_FILE: &str = "/tmp/schemaui.yaml";

#[derive(Debug, Parser)]
#[command(
    name = "schemaui",
    version,
    about = "Render JSON Schemas as interactive TUIs"
)]
struct Cli {
    /// Path to a JSON Schema document ("-" reads from stdin)
    #[arg(short = 's', long = "schema", value_name = "PATH")]
    schema: Option<String>,

    /// Explicit schema format (json/yaml/toml). Defaults to file extension or json.
    #[arg(long = "schema-format", value_name = "FORMAT")]
    schema_format: Option<String>,

    /// Optional config data used to seed defaults ("-" reads from stdin)
    #[arg(short = 'd', long = "data", value_name = "PATH")]
    data: Option<String>,

    /// Explicit data format (json/yaml/toml). Defaults to file extension or json.
    #[arg(long = "data-format", value_name = "FORMAT")]
    data_format: Option<String>,

    /// Title shown at the top of the UI
    #[arg(long = "title", value_name = "TEXT")]
    title: Option<String>,

    /// Output format for the final document (json/yaml/toml)
    #[arg(long = "output-format", value_name = "FORMAT")]
    output_format: Option<String>,

    /// Write the final document to stdout
    #[arg(long = "stdout")]
    stdout: bool,

    /// Additional output file destinations
    #[arg(long = "output", value_name = "PATH")]
    output_files: Vec<PathBuf>,

    /// Override the default temp file location (only used when no other destinations are set)
    #[arg(long = "temp-file", value_name = "PATH")]
    temp_file: Option<PathBuf>,

    /// Disable writing to the default temp file when no destinations are provided
    #[arg(long = "no-temp-file")]
    no_temp_file: bool,

    /// Emit compact JSON/TOML rather than pretty formatting
    #[arg(long = "no-pretty")]
    no_pretty: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let schema_format = resolve_format(cli.schema_format.as_deref(), cli.schema.as_deref())?;
    let data_format = resolve_format(cli.data_format.as_deref(), cli.data.as_deref())?;

    let schema_value = load_optional_value(cli.schema.as_deref(), schema_format, "schema")?;
    let data_value = load_optional_value(cli.data.as_deref(), data_format, "data")?;

    if schema_value.is_none() && data_value.is_none() {
        return Err(eyre!("provide at least --schema or --data"));
    }

    let schema = match (schema_value, data_value.as_ref()) {
        (Some(schema), Some(defaults)) => schema_with_defaults(&schema, defaults),
        (Some(schema), None) => schema,
        (None, Some(defaults)) => schema_from_data_value(defaults),
        (None, None) => unreachable!("validated above"),
    };

    let mut ui = SchemaUI::new(schema);
    if let Some(title) = cli.title.as_ref() {
        ui = ui.with_title(title.clone());
    }
    if let Some(defaults) = data_value.as_ref() {
        ui = ui.with_default_data(defaults);
    }

    let output_settings = build_output_options(&cli)?;
    if let Some(options) = output_settings {
        ui = ui.with_output(options);
    }

    let _ = ui.run().map_err(Report::msg)?;

    Ok(())
}

fn resolve_format(keyword: Option<&str>, path_hint: Option<&str>) -> Result<DocumentFormat> {
    if let Some(value) = keyword {
        return DocumentFormat::from_keyword(value).map_err(Report::msg);
    }
    if let Some(path) = path_hint {
        if path != "-" {
            if let Some(format) = DocumentFormat::from_extension(Path::new(path)) {
                return Ok(format);
            }
        }
    }
    Ok(DocumentFormat::default())
}

fn load_optional_value(
    spec: Option<&str>,
    format: DocumentFormat,
    label: &str,
) -> Result<Option<Value>> {
    match spec {
        Some(path) => load_value(path, format, label).map(Some),
        None => Ok(None),
    }
}

fn load_value(spec: &str, format: DocumentFormat, label: &str) -> Result<Value> {
    let contents = if spec == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .wrap_err("failed to read from stdin")?;
        buffer
    } else {
        fs::read_to_string(spec).wrap_err_with(|| format!("failed to read {label} file {spec}"))?
    };
    let value = parse_document_str(&contents, format)
        .map_err(|err| Report::msg(format!("failed to parse {label} as {}: {err}", format)))?;
    Ok(value)
}

fn build_output_options(cli: &Cli) -> Result<Option<OutputOptions>> {
    let format = if let Some(value) = cli.output_format.as_deref() {
        DocumentFormat::from_keyword(value).map_err(Report::msg)?
    } else {
        DocumentFormat::default()
    };

    let mut destinations = Vec::new();
    if cli.stdout {
        destinations.push(OutputDestination::Stdout);
    }
    for path in &cli.output_files {
        destinations.push(OutputDestination::file(path));
    }

    if destinations.is_empty() {
        if cli.no_temp_file {
            return Ok(None);
        }
        let fallback = cli
            .temp_file
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_TEMP_FILE));
        destinations.push(OutputDestination::file(fallback));
    }

    Ok(Some(OutputOptions {
        format,
        pretty: !cli.no_pretty,
        destinations,
    }))
}
