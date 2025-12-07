use crate::alerts::http::server::BGPAlerterAlert;
use crate::database::models;
use crate::mcp_clients::{self, MCPConnection};
use color_eyre::Result;
use rig::client::ProviderClient;
use rig::completion::Prompt;
use rig::prelude::CompletionClient;
use rig::providers::anthropic;
use sqlx::SqlitePool;

pub struct Chat;

const PREAMBLE: &str = r#"
You are a BGP security analyst assistant helping NOC operators with follow-up questions about BGP alerts.
You have access to tools to gather information about IP prefixes, ASNs, and routing announcements.
You are answering questions about a BGP alert that has already been analyzed. Provide clear, concise answers
based on the original alert data and your access to current routing information.
Do not use emojis in your responses. Use plain text formatting only."#;

impl Chat {
    pub async fn run(
        alert: BGPAlerterAlert,
        initial_response: &str,
        chat_history: &[models::ChatMessage],
        user_question: &str,
        config: &crate::config::AppConfig,
        db_pool: &SqlitePool,
    ) -> Result<String> {
        dotenv::dotenv().ok();

        tracing::info!("Starting chat agent run");

        // Connect to all enabled MCP servers from database
        let mcp_connections = mcp_clients::connect_all_enabled(db_pool).await?;

        if mcp_connections.is_empty() {
            tracing::warn!("No MCP servers available - chat agent will run without tools");
        } else {
            let total_tools: usize = mcp_connections.iter().map(|c| c.tool_count()).sum();
            tracing::info!(
                "Connected to {} MCP server(s) with {} total tool(s)",
                mcp_connections.len(),
                total_tools
            );
        }

        let completion_model = anthropic::Client::from_env();

        // Build context from original alert and chat history
        let alert_json = serde_json::to_string_pretty(&alert)?;

        // Format chat history (last 10-15 messages)
        let recent_history: Vec<_> = chat_history.iter().rev().take(15).rev().collect();
        let chat_context = if recent_history.is_empty() {
            String::new()
        } else {
            let mut context = String::from("\n\nPrevious conversation:\n");
            for msg in recent_history {
                context.push_str(&format!("{}: {}\n", msg.role, msg.content));
            }
            context
        };

        let prompt = format!(
            r#"You are answering a follow-up question about a BGP alert that was previously analyzed.

Original BGP Alert:
{alert_json}

Initial Analysis Report:
{initial_response}
{chat_context}

User's Question: {user_question}

Please provide a clear, concise answer to the user's question. You can use the available tools to gather additional information if needed.
Do not use emojis - use plain text formatting only."#
        );

        // Build and run agent with or without MCP tools
        let res = Self::run_agent_with_tools(
            completion_model,
            &config.llm_model_name,
            mcp_connections,
            &prompt,
        )
        .await?;

        Ok(res)
    }

    async fn run_agent_with_tools(
        client: anthropic::Client,
        model_name: &str,
        connections: Vec<MCPConnection>,
        prompt: &str,
    ) -> Result<String> {
        // Handle the case with no MCP connections
        if connections.is_empty() {
            let agent = client.agent(model_name).preamble(PREAMBLE).build();
            return Ok(agent.prompt(prompt).multi_turn(3).await?);
        }

        // Build agent with MCP tools
        // We need to handle the type transformation that happens when adding rmcp_tools
        let mut connections_iter = connections.into_iter();

        // Start with the first connection
        let first_conn = connections_iter.next().unwrap();
        let mut agent_builder = client
            .agent(model_name)
            .preamble(PREAMBLE)
            .rmcp_tools(first_conn.tools, first_conn.peer);

        // Add remaining connections
        for conn in connections_iter {
            agent_builder = agent_builder.rmcp_tools(conn.tools, conn.peer);
        }

        let agent = agent_builder.build();
        Ok(agent.prompt(prompt).multi_turn(3).await?)
    }
}
