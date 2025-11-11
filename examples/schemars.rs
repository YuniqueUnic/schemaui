use serde_json::json;

fn main() -> anyhow::Result<()> {
    let _schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Service Config",
        "description": "Edit service configuration interactively",
        "type": "object",
        "required": ["host", "port"],
        "properties": {
            "host": {
                "type": "string",
                "description": "Server listening address",
                "default": "127.0.0.1"
            },
            "port": {
                "type": "integer",
                "description": "Server port",
                "default": 8080
            },
            "log": {
                "type": "object",
                "description": "Logging configuration",
                "properties": {
                    "level": {
                        "type": "string",
                        "enum": ["trace", "debug", "info", "warn", "error"],
                        "default": "info"
                    }
                }
            },
            "features": {
                "type": "array",
                "description": "Enabled feature flags",
                "items": { "type": "string" }
            },
            "public": {
                "type": "boolean",
                "description": "Expose service publicly",
                "default": false
            }
        }
    });

    Ok(())
}
