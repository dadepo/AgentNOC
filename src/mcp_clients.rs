use color_eyre::Result;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use rmcp::{Peer, RoleClient, ServiceExt};

use crate::config::RIPESTAT_MCP_ENDPONT;

/// Container for MCP client tools and peer information
/// IMPORTANT: The Box<dyn ...> must be kept alive for the peer to work
pub struct MCPConnection {
    pub tools: Vec<Tool>,
    pub peer: Peer<RoleClient>,
    #[allow(dead_code)] // This field must exist to keep the service alive
    _service: Box<dyn std::any::Any + Send>,
}

/// Connect to the RIPEstat MCP server and return tools and peer
pub async fn connect_ripestat() -> Result<MCPConnection> {
    let client_info = ClientInfo {
        protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "agent_noc_ripestat".to_string(),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
    };

    let http_client = reqwest::Client::new();
    let transport = StreamableHttpClientTransport::with_client(
        http_client,
        StreamableHttpClientTransportConfig {
            uri: RIPESTAT_MCP_ENDPONT.into(),
            ..Default::default()
        },
    );

    tracing::info!("Connecting to RIPEstat MCP server...");
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("RIPEstat client error: {:?}", e);
    })?;

    let server_info = client.peer_info();
    tracing::info!("Connected to RIPEstat: {server_info:#?}");

    let tools_result = client.list_tools(Default::default()).await?;
    tracing::info!(
        "Available RIPEstat tools: {} tool(s)",
        tools_result.tools.len()
    );

    let peer = client.peer().to_owned();

    Ok(MCPConnection {
        tools: tools_result.tools,
        peer,
        _service: Box::new(client), // Keep the service alive!
    })
}

/// Connect to the WHOIS MCP server via stdio and return tools and peer
pub async fn connect_whois() -> Result<MCPConnection> {
    let client_info = ClientInfo {
        protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "agent_noc_whois".to_string(),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
    };

    // Spawn the whois MCP server via uvx
    let mut command = tokio::process::Command::new("uvx");
    command.args([
        "--from",
        "git+https://github.com/dadepo/whois-mcp.git",
        "whois-mcp",
    ]);

    let transport = TokioChildProcess::new(command)?;

    tracing::info!("Connecting to WHOIS MCP server via stdio...");
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("WHOIS client error: {:?}", e);
    })?;

    let server_info = client.peer_info();
    tracing::info!("Connected to WHOIS: {server_info:#?}");

    let tools_result = client.list_tools(Default::default()).await?;
    tracing::info!(
        "Available WHOIS tools: {} tool(s)",
        tools_result.tools.len()
    );

    let peer = client.peer().to_owned();

    Ok(MCPConnection {
        tools: tools_result.tools,
        peer,
        _service: Box::new(client), // Keep the service alive!
    })
}
