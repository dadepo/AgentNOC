use color_eyre::Result;
use rig::completion::Prompt;
use rig::prelude::CompletionClient;
use rig::providers::anthropic;
use rmcp::ServiceExt;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};

use crate::config::AppConfig;

pub async fn run(ip: String, config: &AppConfig) -> Result<String> {
    dotenv::dotenv().ok();

    tracing::info!("Starting health check for IP: {}", ip);

    let transport =
        rmcp::transport::StreamableHttpClientTransport::from_uri(&*config.mcp_server_url);

    let client_info = ClientInfo {
        protocol_version: rmcp::model::ProtocolVersion::V_2024_11_05,
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "noc_agent".to_string(),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
    };

    tracing::info!("Connecting to whois MCP server...");
    let client = client_info.serve(transport).await.inspect_err(|e| {
        eprintln!("Client error: {:?}", e);
        tracing::error!("Client connection error: {:?}", e);
    })?;
    tracing::info!("Successfully connected to MCP server");

    // Initialize
    let server_info = client.peer_info();
    tracing::info!("Connected to endpoints: {server_info:#?}");

    let tools: Vec<rmcp::model::Tool> = client.list_tools(Default::default()).await?.tools;

    let llm_client = anthropic::Client::from_env();
    let agent = llm_client
            .agent(&config.llm_model_name)
            .preamble("You are a helpful assistant who has access to a number of tools from an MCP endpoints designed to be used for incrementing and decrementing a counter.")
            .rmcp_tools(tools, client.peer().to_owned())
            .build();

    let res = agent
        .prompt(format!("What organization owns {ip}"))
        .multi_turn(2)
        .await?;

    Ok(res)
}
