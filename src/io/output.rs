use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use super::DocumentFormat;

/// Destination for serialized output values.
#[derive(Debug, Clone)]
pub enum OutputDestination {
    Stdout,
    File(PathBuf),
}

impl OutputDestination {
    pub fn file(path: impl AsRef<Path>) -> Self {
        OutputDestination::File(path.as_ref().to_path_buf())
    }
}

/// Controls how data is serialized after the UI completes.
#[derive(Debug, Clone)]
pub struct OutputOptions {
    pub format: DocumentFormat,
    pub pretty: bool,
    pub destinations: Vec<OutputDestination>,
}

impl OutputOptions {
    pub fn new(format: DocumentFormat) -> Self {
        Self {
            format,
            pretty: true,
            destinations: vec![OutputDestination::Stdout],
        }
    }

    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    pub fn with_destinations(mut self, destinations: Vec<OutputDestination>) -> Self {
        self.destinations = destinations;
        self
    }

    pub fn add_destination(mut self, destination: OutputDestination) -> Self {
        self.destinations.push(destination);
        self
    }
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self::new(DocumentFormat::Json)
    }
}

/// Serialize and write the final value according to the configured format and destinations.
pub fn emit(value: &Value, options: &OutputOptions) -> Result<()> {
    if options.destinations.is_empty() {
        return Ok(());
    }
    let payload = serialize_value(value, options)?;
    for destination in &options.destinations {
        write_payload(destination, &payload).with_context(|| match destination {
            OutputDestination::Stdout => "failed to write to stdout".to_string(),
            OutputDestination::File(path) => {
                format!("failed to write to file {}", path.display())
            }
        })?;
    }
    Ok(())
}

fn serialize_value(value: &Value, options: &OutputOptions) -> Result<String> {
    match options.format {
        DocumentFormat::Json => {
            if options.pretty {
                serde_json::to_string_pretty(value).context("failed to serialize JSON")
            } else {
                serde_json::to_string(value).context("failed to serialize JSON")
            }
        }
        #[cfg(feature = "yaml")]
        DocumentFormat::Yaml => serde_yaml::to_string(value).context("failed to serialize YAML"),
        #[cfg(feature = "toml")]
        DocumentFormat::Toml => {
            if options.pretty {
                toml::to_string_pretty(value).context("failed to serialize TOML")
            } else {
                toml::to_string(value).context("failed to serialize TOML")
            }
        }
    }
}

fn write_payload(destination: &OutputDestination, payload: &str) -> Result<()> {
    match destination {
        OutputDestination::Stdout => {
            let mut stdout = io::stdout();
            stdout
                .write_all(payload.as_bytes())
                .and_then(|_| stdout.write_all(b"\n"))
                .context("failed to flush stdout")?;
            stdout.flush().context("failed to flush stdout")
        }
        OutputDestination::File(path) => {
            let mut file = File::create(path)?;
            file.write_all(payload.as_bytes())?;
            file.write_all(b"\n")?;
            file.flush()?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn writes_to_stdout_noop_when_not_configured() {
        let options = OutputOptions {
            format: DocumentFormat::Json,
            pretty: true,
            destinations: Vec::new(),
        };
        emit(&json!({"ok": true}), &options).unwrap();
    }

    #[test]
    fn writes_to_file_destination() {
        let dir = std::env::temp_dir();
        let filename = format!(
            "schemaui-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = dir.join(filename);
        let options = OutputOptions {
            format: DocumentFormat::Json,
            pretty: true,
            destinations: vec![OutputDestination::file(&path)],
        };
        emit(&json!({"ok": true}), &options).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"ok\""));
        let _ = fs::remove_file(path);
    }
}
