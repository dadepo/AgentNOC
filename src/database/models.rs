use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Common metadata for all MCP servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDetails {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub is_native: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// MCP Server configuration - enum with transport-specific variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "transport_type", rename_all = "lowercase")]
pub enum McpServer {
    Http {
        #[serde(flatten)]
        meta: McpServerDetails,
        url: String,
    },
    Stdio {
        #[serde(flatten)]
        meta: McpServerDetails,
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
}

impl McpServer {
    /// Get the common metadata
    pub fn meta(&self) -> &McpServerDetails {
        match self {
            McpServer::Http { meta, .. } => meta,
            McpServer::Stdio { meta, .. } => meta,
        }
    }

    /// Get server name
    pub fn name(&self) -> &str {
        &self.meta().name
    }

    /// Create from database row values
    #[allow(clippy::too_many_arguments)]
    pub fn from_row(
        id: i64,
        name: String,
        description: Option<String>,
        transport_type: String,
        url: Option<String>,
        command: Option<String>,
        args: Option<String>,
        env: Option<String>,
        enabled: i64,
        is_native: i64,
        created_at: String,
        updated_at: String,
    ) -> Result<Self, String> {
        let meta = McpServerDetails {
            id,
            name,
            description,
            enabled: enabled != 0,
            is_native: is_native != 0,
            created_at,
            updated_at,
        };

        match transport_type.to_lowercase().as_str() {
            "http" => {
                let url = url.ok_or("HTTP transport requires a URL")?;
                if url.is_empty() {
                    return Err("HTTP transport requires a non-empty URL".to_string());
                }
                Ok(McpServer::Http { meta, url })
            }
            "stdio" => {
                let command = command.ok_or("Stdio transport requires a command")?;
                if command.is_empty() {
                    return Err("Stdio transport requires a non-empty command".to_string());
                }
                let args: Vec<String> = args
                    .map(|s| serde_json::from_str(&s))
                    .transpose()
                    .map_err(|e| format!("Failed to parse args JSON: {e}"))?
                    .unwrap_or_default();
                let env: HashMap<String, String> = env
                    .map(|s| serde_json::from_str(&s))
                    .transpose()
                    .map_err(|e| format!("Failed to parse env JSON: {e}"))?
                    .unwrap_or_default();
                Ok(McpServer::Stdio {
                    meta,
                    command,
                    args,
                    env,
                })
            }
            _ => Err(format!(
                "Invalid transport type: '{transport_type}'. Expected 'http' or 'stdio'"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "transport_type", rename_all = "lowercase")]
pub enum CreateMcpServer {
    Http {
        name: String,
        #[serde(default)]
        description: Option<String>,
        url: String,
        #[serde(default = "default_enabled")]
        enabled: bool,
    },
    Stdio {
        name: String,
        #[serde(default)]
        description: Option<String>,
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(default = "default_enabled")]
        enabled: bool,
    },
}

fn default_enabled() -> bool {
    true
}

impl CreateMcpServer {
    /// Get server name
    pub fn name(&self) -> &str {
        match self {
            CreateMcpServer::Http { name, .. } => name,
            CreateMcpServer::Stdio { name, .. } => name,
        }
    }

    /// Validate the create request
    pub fn validate(&self) -> Result<(), String> {
        match self {
            CreateMcpServer::Http { name, url, .. } => {
                if name.is_empty() {
                    return Err("Name is required".to_string());
                }
                if url.is_empty() {
                    return Err("HTTP transport requires a non-empty URL".to_string());
                }
                Ok(())
            }
            CreateMcpServer::Stdio { name, command, .. } => {
                if name.is_empty() {
                    return Err("Name is required".to_string());
                }
                if command.is_empty() {
                    return Err("Stdio transport requires a non-empty command".to_string());
                }
                Ok(())
            }
        }
    }
}

/// Note: transport_type cannot be changed after creation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateMcpServer {
    pub name: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Alert {
    pub id: i64,
    pub alert_data: Value,
    pub initial_response: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub alert_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

impl Alert {
    #[allow(dead_code)]
    pub fn from_row(
        id: i64,
        alert_data: String,
        initial_response: String,
        created_at: String,
        updated_at: String,
    ) -> Result<Self, serde_json::Error> {
        let alert_data: Value = serde_json::from_str(&alert_data)?;
        Ok(Alert {
            id,
            alert_data,
            initial_response,
            created_at,
            updated_at,
        })
    }
}

impl ChatMessage {
    pub fn from_row(
        id: i64,
        alert_id: i64,
        role: String,
        content: String,
        created_at: String,
    ) -> Self {
        ChatMessage {
            id,
            alert_id,
            role,
            content,
            created_at,
        }
    }
}

pub fn get_current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_from_row_http() {
        let server = McpServer::from_row(
            1,
            "ripestat".to_string(),
            Some("RIPEstat MCP Server".to_string()),
            "http".to_string(),
            Some("https://example.com/mcp".to_string()),
            None,
            None,
            None,
            1,
            0, // is_native
            "2025-01-01T00:00:00Z".to_string(),
            "2025-01-01T00:00:00Z".to_string(),
        )
        .unwrap();

        assert_eq!(server.meta().id, 1);
        assert_eq!(server.name(), "ripestat");
        assert!(server.meta().enabled);

        match server {
            McpServer::Http { url, .. } => {
                assert_eq!(url, "https://example.com/mcp");
            }
            _ => panic!("Expected HTTP variant"),
        }
    }

    #[test]
    fn test_mcp_server_from_row_stdio() {
        let args_json = r#"["--from", "git+https://example.com/mcp.git", "mcp"]"#;
        let env_json = r#"{"KEY": "value"}"#;

        let server = McpServer::from_row(
            2,
            "whois".to_string(),
            Some("WHOIS MCP Server".to_string()),
            "stdio".to_string(),
            None,
            Some("uvx".to_string()),
            Some(args_json.to_string()),
            Some(env_json.to_string()),
            1,
            0, // is_native
            "2025-01-01T00:00:00Z".to_string(),
            "2025-01-01T00:00:00Z".to_string(),
        )
        .unwrap();

        assert_eq!(server.meta().id, 2);
        assert_eq!(server.name(), "whois");

        match server {
            McpServer::Stdio {
                command, args, env, ..
            } => {
                assert_eq!(command, "uvx");
                assert_eq!(
                    args,
                    vec!["--from", "git+https://example.com/mcp.git", "mcp"]
                );
                assert_eq!(env.get("KEY"), Some(&"value".to_string()));
            }
            _ => panic!("Expected Stdio variant"),
        }
    }

    #[test]
    fn test_mcp_server_from_row_invalid_transport() {
        let result = McpServer::from_row(
            1,
            "test".to_string(),
            None,
            "invalid".to_string(),
            None,
            None,
            None,
            None,
            1,
            0, // is_native
            "2025-01-01T00:00:00Z".to_string(),
            "2025-01-01T00:00:00Z".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid transport type"));
    }

    #[test]
    fn test_mcp_server_from_row_http_missing_url() {
        let result = McpServer::from_row(
            1,
            "test".to_string(),
            None,
            "http".to_string(),
            None, // Missing URL
            None,
            None,
            None,
            1,
            0, // is_native
            "2025-01-01T00:00:00Z".to_string(),
            "2025-01-01T00:00:00Z".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("URL"));
    }

    #[test]
    fn test_mcp_server_from_row_stdio_missing_command() {
        let result = McpServer::from_row(
            1,
            "test".to_string(),
            None,
            "stdio".to_string(),
            None,
            None, // Missing command
            None,
            None,
            1,
            0, // is_native
            "2025-01-01T00:00:00Z".to_string(),
            "2025-01-01T00:00:00Z".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("command"));
    }

    #[test]
    fn test_mcp_server_serde_roundtrip_http() {
        let server = McpServer::Http {
            meta: McpServerDetails {
                id: 1,
                name: "test".to_string(),
                description: Some("Test server".to_string()),
                enabled: true,
                is_native: false,
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
            },
            url: "https://example.com".to_string(),
        };

        let json = serde_json::to_string(&server).unwrap();
        assert!(json.contains(r#""transport_type":"http""#));
        assert!(json.contains(r#""url":"https://example.com""#));

        let parsed: McpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.meta().id, 1);
        assert!(matches!(parsed, McpServer::Http { .. }));
    }

    #[test]
    fn test_mcp_server_serde_roundtrip_stdio() {
        let server = McpServer::Stdio {
            meta: McpServerDetails {
                id: 2,
                name: "test".to_string(),
                description: None,
                enabled: true,
                is_native: false,
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
            },
            command: "uvx".to_string(),
            args: vec!["--from".to_string(), "test".to_string()],
            env: HashMap::new(),
        };

        let json = serde_json::to_string(&server).unwrap();
        assert!(json.contains(r#""transport_type":"stdio""#));
        assert!(json.contains(r#""command":"uvx""#));

        let parsed: McpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.meta().id, 2);
        assert!(matches!(parsed, McpServer::Stdio { .. }));
    }

    #[test]
    fn test_create_mcp_server_validate_http() {
        let valid = CreateMcpServer::Http {
            name: "test".to_string(),
            description: None,
            url: "https://example.com".to_string(),
            enabled: true,
        };
        assert!(valid.validate().is_ok());

        let invalid_empty_name = CreateMcpServer::Http {
            name: "".to_string(),
            description: None,
            url: "https://example.com".to_string(),
            enabled: true,
        };
        assert!(invalid_empty_name.validate().is_err());

        let invalid_empty_url = CreateMcpServer::Http {
            name: "test".to_string(),
            description: None,
            url: "".to_string(),
            enabled: true,
        };
        assert!(invalid_empty_url.validate().is_err());
    }

    #[test]
    fn test_create_mcp_server_validate_stdio() {
        let valid = CreateMcpServer::Stdio {
            name: "test".to_string(),
            description: None,
            command: "uvx".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
        };
        assert!(valid.validate().is_ok());

        let invalid_empty_name = CreateMcpServer::Stdio {
            name: "".to_string(),
            description: None,
            command: "uvx".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
        };
        assert!(invalid_empty_name.validate().is_err());

        let invalid_empty_command = CreateMcpServer::Stdio {
            name: "test".to_string(),
            description: None,
            command: "".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
        };
        assert!(invalid_empty_command.validate().is_err());
    }

    #[test]
    fn test_create_mcp_server_serde_http() {
        let json = r#"{
            "transport_type": "http",
            "name": "ripestat",
            "description": "RIPEstat MCP Server",
            "url": "https://example.com/mcp"
        }"#;

        let server: CreateMcpServer = serde_json::from_str(json).unwrap();
        match server {
            CreateMcpServer::Http {
                name,
                description,
                url,
                enabled,
            } => {
                assert_eq!(name, "ripestat");
                assert_eq!(description, Some("RIPEstat MCP Server".to_string()));
                assert_eq!(url, "https://example.com/mcp");
                assert!(enabled); // default value
            }
            _ => panic!("Expected HTTP variant"),
        }
    }

    #[test]
    fn test_create_mcp_server_serde_stdio() {
        let json = r#"{
            "transport_type": "stdio",
            "name": "whois",
            "command": "uvx",
            "args": ["--from", "test"],
            "enabled": false
        }"#;

        let server: CreateMcpServer = serde_json::from_str(json).unwrap();
        match server {
            CreateMcpServer::Stdio {
                name,
                command,
                args,
                enabled,
                ..
            } => {
                assert_eq!(name, "whois");
                assert_eq!(command, "uvx");
                assert_eq!(args, vec!["--from".to_string(), "test".to_string()]);
                assert!(!enabled);
            }
            _ => panic!("Expected Stdio variant"),
        }
    }

    #[test]
    fn test_alert_from_row() {
        let alert_data = r#"{"message":"test","description":"test desc","details":{"prefix":"192.0.2.0/24","asn":"1234"}}"#;
        let initial_response = "Test response";
        let created_at = "2025-01-15T10:30:00Z";
        let updated_at = "2025-01-15T10:30:00Z";

        let alert = Alert::from_row(
            1,
            alert_data.to_string(),
            initial_response.to_string(),
            created_at.to_string(),
            updated_at.to_string(),
        )
        .unwrap();

        assert_eq!(alert.id, 1);
        assert_eq!(alert.initial_response, initial_response);
        assert_eq!(alert.created_at, created_at);
        assert_eq!(alert.updated_at, updated_at);
        assert!(alert.alert_data.is_object());
    }

    #[test]
    fn test_alert_from_row_invalid_json() {
        let result = Alert::from_row(
            1,
            "invalid json".to_string(),
            "response".to_string(),
            "2025-01-15T10:30:00Z".to_string(),
            "2025-01-15T10:30:00Z".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_chat_message_from_row() {
        let msg = ChatMessage::from_row(
            1,
            10,
            "user".to_string(),
            "Hello".to_string(),
            "2025-01-15T10:30:00Z".to_string(),
        );

        assert_eq!(msg.id, 1);
        assert_eq!(msg.alert_id, 10);
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.created_at, "2025-01-15T10:30:00Z");
    }

    #[test]
    fn test_get_current_timestamp() {
        let timestamp = get_current_timestamp();
        assert!(!timestamp.is_empty());
        // Should be valid RFC3339 format
        assert!(timestamp.contains('T'));
        assert!(timestamp.contains('Z') || timestamp.contains('+') || timestamp.contains('-'));
    }
}
