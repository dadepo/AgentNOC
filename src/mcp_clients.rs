use color_eyre::Result;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use rmcp::{Peer, RoleClient, ServiceExt};

/// Container for MCP client tools and peer information
pub struct MCPConnection {
    pub tools: Vec<Tool>,
    pub peer: Peer<RoleClient>,
}

/// Connect to the RIPEstat MCP server and return tools and peer
pub async fn connect_ripestat() -> Result<MCPConnection> {
    let client_info = ClientInfo {
        protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "noc_agent_ripestat".to_string(),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
    };

    let http_client = reqwest::Client::new();
    let transport = StreamableHttpClientTransport::with_client(
        http_client,
        StreamableHttpClientTransportConfig {
            uri: "https://mcp-ripestat.taihen.org/mcp".into(),
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
    })
}

/// Connect to the WHOIS MCP server and return tools and peer
pub async fn connect_whois(uri: &str) -> Result<MCPConnection> {
    let client_info = ClientInfo {
        protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "noc_agent_whois".to_string(),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
    };

    let http_client = reqwest::Client::new();
    let transport = StreamableHttpClientTransport::with_client(
        http_client,
        StreamableHttpClientTransportConfig {
            uri: uri.into(),
            ..Default::default()
        },
    );

    tracing::info!("Connecting to WHOIS MCP server at {}...", uri);
    let client = client_info.serve(transport).await.inspect_err(|e| {
        tracing::error!("WHOIS client error: {:?}", e);
    })?;

    let server_info = client.peer_info();
    tracing::info!("Connected to WHOIS: {server_info:#?}");

    let tools_result = client.list_tools(Default::default()).await?;
    tracing::info!("Available WHOIS tools: {} tool(s)", tools_result.tools.len());

    let peer = client.peer().to_owned();

    Ok(MCPConnection {
        tools: tools_result.tools,
        peer,
    })
}
