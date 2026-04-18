use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use color_eyre::eyre::{Report, Result, WrapErr};
use schemaui::{DocumentFormat, parse_document_str};
use serde_json::Value;

#[derive(Debug)]
pub enum InputSource {
    File(PathBuf),
    Stdin,
}

pub fn load_value(spec: &str, format: DocumentFormat, label: &str) -> Result<Value> {
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
                DocumentFormat::format_list()
            )))
        }
    }
}

pub fn is_not_found(err: &Report) -> bool {
    err.downcast_ref::<io::Error>()
        .is_some_and(|io_err| io_err.kind() == io::ErrorKind::NotFound)
}
