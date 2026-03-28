use std::path::PathBuf;

use schemaui::{DocumentFormat, OutputDestination, OutputOptions};

use crate::cli::CommonArgs;

use super::DEFAULT_TEMP_FILE;
use super::diagnostics::DiagnosticCollector;
use super::format::{ExtensionFormat, probe_format_from_extension};

pub fn build_output_options(
    args: &CommonArgs,
    config_hint: Option<DocumentFormat>,
    schema_hint: Option<DocumentFormat>,
    diagnostics: &mut DiagnosticCollector,
) -> (Option<OutputOptions>, Vec<PathBuf>) {
    let mut destinations = Vec::new();
    let explicit_outputs = !args.outputs.is_empty();

    for raw in &args.outputs {
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
        if args.no_temp_file {
            return (None, Vec::new());
        }
        let fallback = args
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
        infer_format_from_files(&file_paths, diagnostics)
            .or(config_hint)
            .or(schema_hint)
            .unwrap_or_default()
    };

    if diagnostics.len() > start {
        return (None, file_paths);
    }

    let options = OutputOptions {
        destinations,
        pretty: !args.no_pretty,
        format,
    };

    (Some(options), file_paths)
}

pub fn ensure_output_paths_available(
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

fn determine_stdout_format(
    config_hint: Option<DocumentFormat>,
    schema_hint: Option<DocumentFormat>,
) -> DocumentFormat {
    config_hint.or(schema_hint).unwrap_or_default()
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
