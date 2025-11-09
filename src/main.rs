use schemaui::SchemaUI;
use serde_json::json;

type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> AppResult<()> {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Simple Configuration Schema",
        "type": "object",
        "required": ["host", "port"],
        "properties": {
            "host": {
                "type": "string",
                "description": "Server listening address",
                "default": "localhost"
            },
            "port": {
                "type": "integer",
                "description": "Server port",
                "minimum": 1,
                "maximum": 65535,
                "default": 8080
            },
            "log": {
                "type": "object",
                "description": "Log configuration",
                "properties": {
                    "level": {
                        "type": "string",
                        "enum": ["debug", "info", "warn", "error"],
                        "default": "info"
                    }
                }
            },
            "secret": {
                "type": "string",
                "description": "Secret key",
                "minLength": 8
            },
            "tags": {
                "type": "array",
                "description": "Service tags",
                "items": {
                    "type": "string"
                },
                "default": ["web", "api"]
            }
        },
        "additionalProperties": false
    });

    let value = SchemaUI::new(schema).with_title("SchemaUI Demo").run()?;

    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
