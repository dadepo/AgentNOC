use color_eyre::Result;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use rmcp::{Peer, RoleClient, ServiceExt};
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::database::db::get_enabled_mcp_servers;
use crate::database::models::McpServer;

/// Container for MCP client tools and peer information
/// IMPORTANT: The Box<dyn ...> must be kept alive for the peer to work
pub struct MCPConnection {
    pub name: String,
    pub tools: Vec<Tool>,
    pub peer: Peer<RoleClient>,
    #[allow(dead_code)] // This field must exist to keep the service alive
    _service: Box<dyn std::any::Any + Send>,
}

impl MCPConnection {
    /// Get the number of tools available in this connection
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

/// Connect to an MCP server based on its configuration
pub async fn connect(server: &McpServer) -> Result<MCPConnection> {
    let client_info = ClientInfo {
        protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: format!("agent_noc_{}", server.name()),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
    };

    match server {
        McpServer::Http { meta, url } => connect_http(&meta.name, client_info, url).await,
        McpServer::Stdio {
            meta,
            command,
            args,
            env,
        } => connect_stdio(&meta.name, client_info, command, args, Some(env)).await,
    }
}

/// Connect to an MCP server using HTTP streamable transport
async fn connect_http(name: &str, client_info: ClientInfo, url: &str) -> Result<MCPConnection> {
    let http_client = reqwest::Client::new();
    let transport = StreamableHttpClientTransport::with_client(
        http_client,
        StreamableHttpClientTransportConfig {
            uri: url.into(),
            ..Default::default()
        },
    );

    tracing::info!("Connecting to {} MCP server (HTTP: {})...", name, url);
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("{} client error: {:?}", name, e);
    })?;

    let server_info = client.peer_info();
    tracing::info!("Connected to {}: {server_info:#?}", name);

    let tools_result = client.list_tools(Default::default()).await?;
    tracing::info!(
        "Available {} tools: {} tool(s)",
        name,
        tools_result.tools.len()
    );

    let peer = client.peer().to_owned();

    Ok(MCPConnection {
        name: name.to_string(),
        tools: tools_result.tools,
        peer,
        _service: Box::new(client),
    })
}

/// Connect to an MCP server using stdio transport
async fn connect_stdio(
    name: &str,
    client_info: ClientInfo,
    command: &str,
    args: &[String],
    env: Option<&HashMap<String, String>>,
) -> Result<MCPConnection> {
    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args);

    if let Some(env_vars) = env {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    let transport = TokioChildProcess::new(cmd)?;

    tracing::info!(
        "Connecting to {} MCP server (stdio: {} {})...",
        name,
        command,
        args.join(" ")
    );
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("{} client error: {:?}", name, e);
    })?;

    let server_info = client.peer_info();
    tracing::info!("Connected to {}: {server_info:#?}", name);

    let tools_result = client.list_tools(Default::default()).await?;
    tracing::info!(
        "Available {} tools: {} tool(s)",
        name,
        tools_result.tools.len()
    );

    let peer = client.peer().to_owned();

    Ok(MCPConnection {
        name: name.to_string(),
        tools: tools_result.tools,
        peer,
        _service: Box::new(client),
    })
}

/// Connect to all enabled MCP servers from the database
///
/// This function attempts to connect to all enabled MCP servers.
/// If a server fails to connect, it logs the error and continues with the rest.
/// Returns a vector of successfully connected servers.
pub async fn connect_all_enabled(pool: &SqlitePool) -> Result<Vec<MCPConnection>> {
    let servers = get_enabled_mcp_servers(pool).await?;

    if servers.is_empty() {
        tracing::warn!("No enabled MCP servers found in database");
        return Ok(Vec::new());
    }

    tracing::info!("Connecting to {} enabled MCP server(s)...", servers.len());

    let mut connections = Vec::new();
    let mut failed_count = 0;

    for server in servers {
        match connect(&server).await {
            Ok(conn) => {
                tracing::info!(
                    "Successfully connected to MCP server '{}' ({} tools)",
                    conn.name,
                    conn.tool_count()
                );
                connections.push(conn);
            }
            Err(e) => {
                tracing::error!(
                    "Failed to connect to MCP server '{}': {:?}",
                    server.name(),
                    e
                );
                failed_count += 1;
            }
        }
    }

    if failed_count > 0 {
        tracing::warn!(
            "{} MCP server(s) failed to connect, {} connected successfully",
            failed_count,
            connections.len()
        );
    }

    Ok(connections)
}

/// Test connection to a specific MCP server
///
/// Returns Ok(tool_count) if connection successful, Err if failed
pub async fn test_connection(server: &McpServer) -> Result<usize> {
    let conn = connect(server).await?;
    Ok(conn.tool_count())
}

#[cfg(test)]
mod tests {
    use crate::database::models::{McpServer, McpServerDetails};

    fn create_test_http_server() -> McpServer {
        McpServer::Http {
            meta: McpServerDetails {
                id: 1,
                name: "test-http".to_string(),
                description: Some("Test HTTP server".to_string()),
                enabled: true,
                is_native: false,
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
            },
            url: "https://example.com/mcp".to_string(),
        }
    }

    fn create_test_stdio_server() -> McpServer {
        McpServer::Stdio {
            meta: McpServerDetails {
                id: 2,
                name: "test-stdio".to_string(),
                description: Some("Test stdio server".to_string()),
                enabled: true,
                is_native: false,
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
            },
            command: "echo".to_string(),
            args: vec!["hello".to_string()],
            env: [("TEST".to_string(), "value".to_string())]
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn test_http_server_fields() {
        let server = create_test_http_server();
        assert_eq!(server.name(), "test-http");
        assert!(server.meta().enabled);

        match server {
            McpServer::Http { url, .. } => {
                assert_eq!(url, "https://example.com/mcp");
            }
            _ => panic!("Expected HTTP variant"),
        }
    }

    #[test]
    fn test_stdio_server_fields() {
        let server = create_test_stdio_server();
        assert_eq!(server.name(), "test-stdio");
        assert!(server.meta().enabled);

        match server {
            McpServer::Stdio {
                command, args, env, ..
            } => {
                assert_eq!(command, "echo");
                assert_eq!(args, vec!["hello".to_string()]);
                assert_eq!(env.get("TEST"), Some(&"value".to_string()));
            }
            _ => panic!("Expected Stdio variant"),
        }
    }
}
