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
          "type": "object",
          "description": "Authentication and authorization settings.",
          "properties": {
            "provider": {
              "type": "string",
              "enum": ["jwt", "oauth2", "basic"],
              "default": "jwt"
            },
            "jwtSecret": {
              "type": "string",
              "minLength": 32,
              "description": "Secret key for signing JWT tokens. Required if provider is 'jwt'."
            },
            "oauth2": {
              "type": "object",
              "properties": {
                "issuerUrl": { "type": "string", "format": "uri" },
                "clientId": { "type": "string" },
                "clientSecret": { "type": "string" }
              }
            }
          },
          "dependencies": {
            "jwtSecret": ["provider"]
          }
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
          "description": "Flags to enable/disable experimental or conditional features.",
          "properties": {
            "enableNewDashboard": {
              "type": "boolean",
              "default": false
            },
            "betaApiAccess": {
              "type": ["boolean", "null"],
              "description": "Grants access to the beta API. 'null' means the feature is controlled by a remote service."
            },
            "allowedOrigins": {
              "type": "array",
              "description": "List of allowed origins for CORS.",
              "items": {
                "type": "string",
                "format": "uri"
              },
              "default": []
            }
          }
        },

      }
    }
    );

    let value = SchemaUI::new(schema).with_title("SchemaUI Demo").run()?;

    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
