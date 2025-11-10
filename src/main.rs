use schemaui::SchemaUI;
use serde_json::json;

type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> AppResult<()> {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Unified Platform Configuration",
        "description": "A full-featured configuration schema showcasing common JSON-Schema structures.",
        "type": "object",
        "definitions": {
            "duration": {
                "type": "object",
                "required": ["unit", "value"],
                "properties": {
                    "unit": {"type": "string", "enum": ["ms", "s", "m", "h"]},
                    "value": {"type": "number", "minimum": 0.0}
                },
                "additionalProperties": false
            },
            "endpoint": {
                "type": "object",
                "required": ["scheme", "host", "port"],
                "properties": {
                    "scheme": {"type": "string", "enum": ["http", "https", "grpc"]},
                    "host": {"type": "string", "format": "hostname"},
                    "port": {"type": "integer", "minimum": 1, "maximum": 65535},
                    "metadata": {
                        "type": "object",
                        "additionalProperties": {"type": "string"}
                    }
                },
                "additionalProperties": false
            },
            "jobBase": {
                "type": "object",
                "required": ["name", "cron"],
                "properties": {
                    "name": {"type": "string", "minLength": 3},
                    "cron": {"type": "string", "pattern": "^([*/0-9,-]+\\s){4}[*/0-9,-]+$"},
                    "enabled": {"type": "boolean", "default": true}
                },
                "additionalProperties": false
            },
            "alertChannel": {
                "type": "object",
                "required": ["type"],
                "oneOf": [
                    {
                        "title": "Slack",
                        "properties": {
                            "type": {"const": "slack"},
                            "webhook": {"type": "string", "format": "uri"},
                            "channel": {"type": "string"}
                        },
                        "required": ["webhook"]
                    },
                    {
                        "title": "PagerDuty",
                        "properties": {
                            "type": {"const": "pagerduty"},
                            "routingKey": {"type": "string", "minLength": 10}
                        },
                        "required": ["routingKey"]
                    },
                    {
                        "title": "Email",
                        "properties": {
                            "type": {"const": "email"},
                            "address": {"type": "string", "format": "email"}
                        },
                        "required": ["address"]
                    }
                ]
            }
        },
        "required": ["metadata", "runtime", "dataPlane", "notifications"],
        "properties": {
            "metadata": {
                "type": "object",
                "required": ["serviceName", "environment"],
                "properties": {
                    "serviceName": {
                        "type": "string",
                        "pattern": "^[a-z0-9-]+$",
                        "minLength": 3,
                        "maxLength": 40
                    },
                    "environment": {
                        "type": "string",
                        "enum": ["dev", "qa", "staging", "prod"]
                    },
                    "version": {"type": "string", "default": "1.0.0"},
                    "owners": {
                        "type": "array",
                        "minItems": 1,
                        "items": {"type": "string", "format": "email"}
                    },
                    "labels": {
                        "type": "object",
                        "patternProperties": {
                            "^[a-z0-9_.-]+$": {"type": "string", "maxLength": 64}
                        },
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            },
            "runtime": {
                "type": "object",
                "required": ["http", "limits"],
                "properties": {
                    "http": {
                        "type": "object",
                        "properties": {
                            "host": {"type": "string", "format": "ipv4", "default": "0.0.0.0"},
                            "port": {"type": "integer", "minimum": 1024, "maximum": 65535},
                            "enableTls": {"type": "boolean", "default": false},
                            "cors": {
                                "type": "object",
                                "properties": {
                                    "allowedOrigins": {
                                        "type": "array",
                                        "items": {"type": "string", "format": "uri"},
                                        "default": []
                                    },
                                    "allowCredentials": {"type": "boolean", "default": false}
                                },
                                "additionalProperties": false
                            }
                        },
                        "additionalProperties": false
                    },
                    "limits": {
                        "type": "object",
                        "properties": {
                            "requestTimeout": {"$ref": "#/definitions/duration"},
                            "maxConnections": {"type": "integer", "minimum": 1, "maximum": 100_000},
                            "burst": {"type": "number", "minimum": 0.0}
                        },
                        "additionalProperties": false
                    },
                    "schedule": {
                        "type": "array",
                        "items": {
                            "allOf": [
                                {"$ref": "#/definitions/jobBase"},
                                {
                                    "type": "object",
                                    "properties": {
                                        "retry": {"type": "integer", "minimum": 0, "maximum": 10},
                                        "action": {
                                            "oneOf": [
                                                {
                                                    "title": "Invoke HTTP",
                                                    "properties": {
                                                        "type": {"const": "http"},
                                                        "endpoint": {"$ref": "#/definitions/endpoint"}
                                                    },
                                                    "required": ["endpoint"]
                                                },
                                                {
                                                    "title": "Publish Event",
                                                    "properties": {
                                                        "type": {"const": "event"},
                                                        "topic": {"type": "string"},
                                                        "payloadTemplate": {"type": "string"}
                                                    },
                                                    "required": ["topic"]
                                                }
                                            ]
                                        }
                                    },
                                    "required": ["action"],
                                    "additionalProperties": false
                                }
                            ]
                        },
                        "default": []
                    }
                },
                "additionalProperties": false
            },
            "dataPlane": {
                "type": "object",
                "required": ["primary"],
                "properties": {
                    "primary": {
                        "type": "object",
                        "oneOf": [
                            {
                                "title": "SQL Cluster",
                                "properties": {
                                    "engine": {"const": "sql"},
                                    "config": {
                                        "type": "object",
                                        "required": ["driver", "dsn"],
                                        "properties": {
                                            "driver": {"type": "string", "enum": ["postgres", "mysql"]},
                                            "dsn": {"type": "string", "minLength": 10},
                                            "maxPool": {"type": "integer", "minimum": 1, "maximum": 200}
                                        },
                                        "additionalProperties": false
                                    }
                                },
                                "required": ["engine", "config"]
                            },
                            {
                                "title": "Document Store",
                                "properties": {
                                    "engine": {"const": "document"},
                                    "config": {
                                        "type": "object",
                                        "required": ["provider", "endpoints"],
                                        "properties": {
                                            "provider": {"type": "string", "enum": ["mongodb", "dynamodb"]},
                                            "endpoints": {
                                                "type": "array",
                                                "items": {"$ref": "#/definitions/endpoint"},
                                                "minItems": 1
                                            },
                                            "replicaSet": {"type": "string"}
                                        },
                                        "additionalProperties": false
                                    }
                                },
                                "required": ["engine", "config"]
                            }
                        ]
                    },
                    "replicas": {
                        "type": "array",
                        "items": {"$ref": "#/definitions/endpoint"},
                        "default": []
                    }
                },
                "additionalProperties": false
            },
            "notifications": {
                "type": "object",
                "properties": {
                    "channels": {
                        "type": "array",
                        "items": {"$ref": "#/definitions/alertChannel"},
                        "minItems": 0
                    },
                    "templates": {
                        "type": "object",
                        "propertyNames": {"pattern": "^[a-z0-9-]+$"},
                        "additionalProperties": {
                            "type": "object",
                            "required": ["subject", "body"],
                            "properties": {
                                "subject": {"type": "string"},
                                "body": {"type": "string", "minLength": 10},
                                "locale": {"type": "string", "pattern": "^[a-z]{2}-[A-Z]{2}$", "default": "en-US"}
                            },
                            "additionalProperties": false
                        }
                    }
                },
                "additionalProperties": false
            },
            "featureFlags": {
                "type": "object",
                "description": "Dynamic feature flags expressed as key/value entries.",
                "additionalProperties": {
                    "type": "object",
                    "required": ["kind", "settings"],
                    "properties": {
                        "kind": {"type": "string", "enum": ["toggle", "percentage", "segment"]},
                        "settings": {
                            "oneOf": [
                                {
                                    "title": "Boolean Toggle",
                                    "properties": {
                                        "state": {"type": "boolean"}
                                    },
                                    "required": ["state"],
                                    "additionalProperties": false
                                },
                                {
                                    "title": "Percentage Rollout",
                                    "properties": {
                                        "percentage": {"type": "number", "minimum": 0, "maximum": 100}
                                    },
                                    "required": ["percentage"],
                                    "additionalProperties": false
                                },
                                {
                                    "title": "Segment List",
                                    "properties": {
                                        "segments": {
                                            "type": "array",
                                            "items": {"type": "string"},
                                            "minItems": 1
                                        }
                                    },
                                    "required": ["segments"],
                                    "additionalProperties": false
                                }
                            ]
                        },
                        "description": {"type": "string"}
                    },
                    "additionalProperties": false
                }
            },
            "secrets": {
                "type": "object",
                "propertyNames": {"pattern": "^[A-Z0-9_]+$"},
                "additionalProperties": {
                    "type": "object",
                    "required": ["type", "payload"],
                    "properties": {
                        "type": {"type": "string", "enum": ["inline", "env", "kms"]},
                        "payload": {"type": "string", "minLength": 4},
                        "version": {"type": "integer", "minimum": 1}
                    },
                    "additionalProperties": false
                }
            },
            "extensions": {
                "type": "array",
                "items": {"type": "string", "pattern": "^[a-z0-9-]+$"},
                "maxItems": 16,
                "default": []
            }
        },
        "additionalProperties": false
    });

    let value = SchemaUI::new(schema).with_title("SchemaUI Demo").run()?;

    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
