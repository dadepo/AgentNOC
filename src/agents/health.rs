use color_eyre::Result;
use rig::completion::Prompt;
use rig::prelude::CompletionClient;
use rig::providers::anthropic;
use rmcp::ServiceExt;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};

pub async fn run(ip: String) -> Result<String> {
    dotenv::dotenv().ok();

    let transport =
        rmcp::transport::StreamableHttpClientTransport::from_uri("http://127.0.0.1:8000/mcp");

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "rig-core".to_string(),
            version: "0.13.0".to_string(),
            ..Default::default()
        },
    };

    let client = client_info.serve(transport).await.inspect_err(|e| {
        eprintln!("Client error: {:?}", e);
    })?;

    // Initialize
    let server_info = client.peer_info();
    println!("Connected to endpoints!");
    tracing::info!("Connected to endpoints: {server_info:#?}");

    println!("Listing available tools...");
    let tools: Vec<rmcp::model::Tool> = client.list_tools(Default::default()).await?.tools;

    // takes the `OPENAI_API_KEY` as an env var on usage
    let llm_client = anthropic::Client::from_env();
    println!("Building agent...");
    let agent = llm_client
            .agent("claude-sonnet-4-5")
            .preamble("You are a helpful assistant who has access to a number of tools from an MCP endpoints designed to be used for incrementing and decrementing a counter.")
            .rmcp_tools(tools, client.peer().to_owned())
            .build();

    println!("Sending prompt to agent...");
    let res = agent
        .prompt(format!("What organization owns {ip}"))
        .multi_turn(2)
        .await?;

    Ok(res)
}
