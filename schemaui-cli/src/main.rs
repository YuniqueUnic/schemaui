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

    /// Inline JSON Schema contents (mutually exclusive with --schema)
    #[arg(long = "schema-inline", value_name = "TEXT", conflicts_with = "schema")]
    schema_inline: Option<String>,

    /// Explicit schema format (json/yaml/toml). Defaults to file extension or json.
    #[arg(long = "schema-format", value_name = "FORMAT")]
    schema_format: Option<String>,

    /// Optional config data used to seed defaults ("-" reads from stdin)
    #[arg(short = 'c', long = "config", alias = "data", value_name = "PATH")]
    config: Option<String>,

    /// Inline config snapshot (mutually exclusive with --config)
    #[arg(
        long = "config-inline",
        alias = "data-inline",
        value_name = "TEXT",
        conflicts_with = "config"
    )]
    config_inline: Option<String>,

    /// Explicit config format (json/yaml/toml). Defaults to file extension or json.
    #[arg(long = "config-format", alias = "data-format", value_name = "FORMAT")]
    config_format: Option<String>,

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

    if cli.schema_inline.is_none()
        && cli.config_inline.is_none()
        && cli.schema.as_deref() == Some("-")
        && cli.config.as_deref() == Some("-")
    {
        return Err(eyre!(
            "cannot read schema and config from stdin at the same time; use --schema-inline or --config-inline"
        ));
    }

    let (schema_format, schema_source) =
        resolve_format(cli.schema_format.as_deref(), cli.schema.as_deref())?;
    let (config_format, config_source) =
        resolve_format(cli.config_format.as_deref(), cli.config.as_deref())?;

    let schema_value = load_optional_value(
        cli.schema.as_deref(),
        cli.schema_inline.as_deref(),
        schema_format,
        schema_source,
        "schema",
        Some(false),
    )?;
    let config_value = load_optional_value(
        cli.config.as_deref(),
        cli.config_inline.as_deref(),
        config_format,
        config_source,
        "config",
        None,
    )?;

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

    let output_settings = build_output_options(&cli)?;
    if let Some(options) = output_settings {
        ui = ui.with_output(options);
    }

    let _ = ui.run().map_err(Report::msg)?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatSource {
    Explicit,
    Extension,
    Default,
}

fn resolve_format(
    keyword: Option<&str>,
    path_hint: Option<&str>,
) -> Result<(DocumentFormat, FormatSource)> {
    if let Some(value) = keyword {
        return DocumentFormat::from_keyword(value)
            .map(|format| (format, FormatSource::Explicit))
            .map_err(Report::msg);
    }
    if let Some(path) = path_hint {
        if path != "-" {
            if let Some(format) = DocumentFormat::from_extension(Path::new(path)) {
                return Ok((format, FormatSource::Extension));
            }
        }
    }
    Ok((DocumentFormat::default(), FormatSource::Default))
}

fn load_optional_value(
    spec: Option<&str>,
    inline: Option<&str>,
    format: DocumentFormat,
    format_source: FormatSource,
    label: &str,
    allow_guess_override: Option<bool>,
) -> Result<Option<Value>> {
    let allow_guess = allow_guess_override.unwrap_or(format_source != FormatSource::Explicit);
    if let Some(contents) = inline {
        return parse_contents(contents, format, label, allow_guess).map(Some);
    }
    match spec {
        Some(path) => load_value(path, format, label, allow_guess).map(Some),
        None => Ok(None),
    }
}

fn load_value(spec: &str, format: DocumentFormat, label: &str, allow_guess: bool) -> Result<Value> {
    let contents = if spec == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .wrap_err("failed to read from stdin")?;
        buffer
    } else {
        fs::read_to_string(spec).wrap_err_with(|| format!("failed to read {label} file {spec}"))?
    };
    parse_contents(&contents, format, label, allow_guess)
}

fn parse_contents(
    contents: &str,
    format: DocumentFormat,
    label: &str,
    allow_guess: bool,
) -> Result<Value> {
    match parse_document_str(contents, format) {
        Ok(value) => Ok(value),
        Err(primary) if allow_guess => {
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
        Err(err) => Err(Report::msg(format!(
            "failed to parse {label} as {}: {err}",
            format
        ))),
    }
}

fn format_list() -> String {
    let items: Vec<String> = DocumentFormat::available_formats()
        .into_iter()
        .map(|fmt| fmt.to_string())
        .collect();
    items.join(", ")
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
