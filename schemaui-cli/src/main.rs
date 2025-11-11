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

    /// Title shown at the top of the UI
    #[arg(long = "title", value_name = "TEXT")]
    title: Option<String>,

    /// Write the final document to stdout
    #[arg(long = "stdout")]
    stdout: bool,

    /// Additional output file destinations
    #[arg(short = 'o', long = "output", value_name = "PATH")]
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

    /// Overwrite output files even if they already exist
    #[arg(short = 'f', long = "force", short_alias = 'y', alias = "yes")]
    force: bool,
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

    let schema_hint = resolve_format_hint(cli.schema.as_deref());
    let config_hint = resolve_format_hint(cli.config.as_deref());

    let schema_value = load_optional_value(
        cli.schema.as_deref(),
        cli.schema_inline.as_deref(),
        schema_hint.format,
        "schema",
    )?;
    let config_value = load_optional_value(
        cli.config.as_deref(),
        cli.config_inline.as_deref(),
        config_hint.format,
        "config",
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

    let (output_settings, output_paths) = build_output_options(
        &cli,
        config_hint.extension_value(),
        schema_hint.extension_value(),
    )?;
    ensure_output_paths_available(&output_paths, cli.force)?;
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

impl FormatHint {
    fn extension_value(&self) -> Option<DocumentFormat> {
        self.from_extension.then_some(self.format)
    }
}

fn resolve_format_hint(path_hint: Option<&str>) -> FormatHint {
    if let Some(path) = path_hint {
        if path != "-" {
            if let Some(format) = DocumentFormat::from_extension(Path::new(path)) {
                return FormatHint {
                    format,
                    from_extension: true,
                };
            }
        }
    }
    FormatHint {
        format: DocumentFormat::default(),
        from_extension: false,
    }
}

fn load_optional_value(
    spec: Option<&str>,
    inline: Option<&str>,
    format: DocumentFormat,
    label: &str,
) -> Result<Option<Value>> {
    if let Some(contents) = inline {
        return parse_contents(contents, format, label).map(Some);
    }
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
    parse_contents(&contents, format, label)
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

fn format_list() -> String {
    let items: Vec<String> = DocumentFormat::available_formats()
        .into_iter()
        .map(|fmt| fmt.to_string())
        .collect();
    items.join(", ")
}

fn build_output_options(
    cli: &Cli,
    config_hint: Option<DocumentFormat>,
    schema_hint: Option<DocumentFormat>,
) -> Result<(Option<OutputOptions>, Vec<PathBuf>)> {
    let mut destinations = Vec::new();
    let mut files = Vec::new();

    if cli.stdout {
        destinations.push(OutputDestination::Stdout);
    }
    for path in &cli.output_files {
        files.push(path.clone());
        destinations.push(OutputDestination::file(path));
    }

    if files.is_empty() && !cli.stdout {
        if cli.no_temp_file {
            return Ok((None, files));
        }
        let fallback = cli
            .temp_file
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_TEMP_FILE));
        files.push(fallback.clone());
        destinations.push(OutputDestination::file(fallback));
    }

    if destinations.is_empty() {
        return Ok((None, files));
    }

    let stdout_only = files.is_empty() && cli.stdout;
    let format = infer_output_format(&files, stdout_only, config_hint, schema_hint)?;

    Ok((
        Some(OutputOptions {
            format,
            pretty: !cli.no_pretty,
            destinations,
        }),
        files,
    ))
}

fn infer_output_format(
    file_paths: &[PathBuf],
    stdout_only: bool,
    config_hint: Option<DocumentFormat>,
    schema_hint: Option<DocumentFormat>,
) -> Result<DocumentFormat> {
    if let Some((first, rest)) = file_paths.split_first() {
        let format = infer_format_from_filename(first)?;
        for path in rest {
            let next = infer_format_from_filename(path)?;
            if next != format {
                return Err(eyre!(
                    "output files use mixed formats; expected all extensions to match {}",
                    format
                ));
            }
        }
        return Ok(format);
    }
    if stdout_only {
        if let Some(format) = config_hint.or(schema_hint) {
            return Ok(format);
        }
    }
    Ok(DocumentFormat::default())
}

fn infer_format_from_filename(path: &Path) -> Result<DocumentFormat> {
    DocumentFormat::from_extension(path).ok_or_else(|| {
        eyre!(
            "cannot infer format from output file {} (use .json/.yaml/.toml)",
            path.display()
        )
    })
}

fn ensure_output_paths_available(paths: &[PathBuf], force: bool) -> Result<()> {
    if force {
        return Ok(());
    }
    for path in paths {
        if path.exists() {
            return Err(eyre!(
                "failed to write: file {} already exists (pass --force to overwrite)",
                path.display()
            ));
        }
    }
    Ok(())
}
