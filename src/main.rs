use schemaui::SchemaUI;
use serde_json::json;

type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> AppResult<()> {
    let schema = json!({
      "$schema": "http://json-schema.org/draft-07/schema#",
      "title": "Omni-Platform Service Configuration",
      "description": "A comprehensive configuration schema for a modern, multi-faceted backend service, including server, database, authentication, logging, and feature flags.",
      "type": "object",
      "required": [
        "serviceName",
        "environment",
        "server",
        "dataStore"
      ],
      "properties": {
        "serviceName": {
          "type": "string",
          "description": "The unique name of this service instance.",
          "pattern": "^[a-z0-9-]+$"
        },
        "environment": {
          "type": "string",
          "description": "The deployment environment.",
          "enum": ["development", "staging", "production"]
        },
        "server": {
          "type": "object",
          "description": "HTTP server configuration.",
          "required": ["port"],
          "properties": {
            "host": {
              "type": "string",
              "default": "0.0.0.0",
              "format": "ipv4"
            },
            "port": {
              "type": "integer",
              "description": "The main port for the HTTP server.",
              "minimum": 1024,
              "maximum": 65535
            },
            "grpcPort": {
                "type": "integer",
                "description": "Optional port for the gRPC server.",
                "minimum": 1024,
                "maximum": 65535
            },
            "timeoutSeconds": {
              "type": "number",
              "description": "Request timeout in seconds, can be a fraction.",
              "minimum": 0.1,
              "default": 30.0
            },
            "enableTls": {
              "type": "boolean",
              "default": false
            }
          }
        },
        "dataStore": {
          "type": "object",
          "description": "Primary data store configuration.",
          "required": ["type"],
          "oneOf": [
            {
              "title": "SQL Database",
              "properties": {
                "type": {
                  "const": "sql"
                },
                "connection": {
                  "type": "object",
                  "required": ["driver", "dsn"],
                  "properties": {
                    "driver": {
                      "type": "string",
                      "enum": ["postgres", "mysql"]
                    },
                    "dsn": {
                      "type": "string",
                      "description": "Data Source Name for the SQL database.",
                      "format": "uri",
                      "minLength": 10
                    },
                    "maxOpenConns": {
                      "type": "integer",
                      "default": 25,
                      "minimum": 1,
                      "maximum": 100
                    }
                  }
                }
              }
            },
            {
              "title": "NoSQL Database",
              "properties": {
                "type": {
                  "const": "nosql"
                },
                "connection": {
                  "type": "object",
                  "required": ["engine", "endpoints"],
                  "properties": {
                    "engine": {
                      "type": "string",
                      "enum": ["mongodb", "dynamodb"]
                    },
                    "endpoints": {
                      "type": "array",
                      "items": {
                        "type": "string",
                        "format": "hostname"
                      },
                      "minItems": 1
                    },
                    "replicaSet": {
                      "type": "string"
                    }
                  }
                }
              }
            }
          ]
        },
        "auth": {
          "title": "Authentication Strategy",
          "description": "Configuration for the authentication mechanism. Only one strategy can be active.",
          "oneOf": [
            {
              "title": "JWT Authentication",
              "required": ["provider", "jwtSecret"],
              "properties": {
                "provider": { "const": "jwt" },
                "jwtSecret": { "type": "string", "minLength": 32, "description": "Secret key for signing JWT tokens." }
              }
            },
            {
              "title": "OAuth2 Authentication",
              "required": ["provider", "oauth2"],
              "properties": {
                "provider": { "const": "oauth2" },
                "oauth2": {
                  "type": "object",
                  "required": ["issuerUrl", "clientId"],
                  "properties": {
                    "issuerUrl": { "type": "string", "format": "uri" },
                    "clientId": { "type": "string" },
                    "clientSecret": { "type": "string" }
                  }
                }
              }
            },
            {
              "title": "Basic Authentication",
              "required": ["provider"],
              "properties": {
                "provider": { "const": "basic" }
              }
            }
          ]
        },
        "logging": {
          "type": "object",
          "properties": {
            "level": {
              "type": "string",
              "enum": ["trace", "debug", "info", "warn", "error"],
              "default": "info"
            },
            "outputs": {
              "type": "array",
              "description": "A list of logging outputs. Can be console, file, or a remote service.",
              "items": {
                "type": "object",
                "required": ["type"],
                "oneOf": [
                  {
                    "properties": {
                      "type": { "const": "console" },
                      "colored": { "type": "boolean", "default": true }
                    }
                  },
                  {
                    "properties": {
                      "type": { "const": "file" },
                      "path": { "type": "string" },
                      "maxSizeMB": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["path"]
                  }
                ]
              }
            }
          }
        },
        "featureFlags": {
          "type": "object",
          "description": "A dynamic set of feature flags. The key is the flag's name, and the value can be a boolean, a number (for percentage rollouts), or an array of strings (for whitelists).",
          "additionalProperties": {
            "anyOf": [
              {
                "type": "boolean",
                "description": "A simple on/off switch."
              },
              {
                "type": "number",
                "minimum": 0,
                "maximum": 100,
                "description": "A percentage for a staged rollout (0-100)."
              },
              {
                "type": "array",
                "items": {
                  "type": "string"
                },
                "description": "A list of user IDs or identifiers to whitelist."
              }
            ]
          }
        }
      }
    });

    let value = SchemaUI::new(schema).with_title("SchemaUI Demo").run()?;

    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
