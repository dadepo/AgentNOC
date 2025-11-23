use crate::alerts::http::server::BGPAlerterAlert;
use crate::database::models;
use crate::mcp_clients;
use color_eyre::Result;
use rig::completion::Prompt;
use rig::prelude::CompletionClient;
use rig::providers::anthropic;

pub struct ChatAgent;

impl ChatAgent {
    pub async fn run(
        alert: BGPAlerterAlert,
        initial_response: &str,
        chat_history: &[models::ChatMessage],
        user_question: &str,
        config: &crate::config::AppConfig,
    ) -> Result<String> {
        dotenv::dotenv().ok();

        tracing::info!("Starting chat agent run");

        // Connect to RIPEstat MCP server
        let ripestat_conn = mcp_clients::connect_ripestat().await?;

        // Connect to WHOIS MCP server via stdio
        let whois_conn = mcp_clients::connect_whois().await?;

        let completion_model = anthropic::Client::from_env();

        let agent = completion_model
            .agent(&config.llm_model_name)
            .preamble(r#"
            You are a BGP security analyst assistant helping NOC operators with follow-up questions about BGP alerts.
            You have access to tools to gather information about IP prefixes, ASNs, and routing announcements.
            You are answering questions about a BGP alert that has already been analyzed. Provide clear, concise answers
            based on the original alert data and your access to current routing information.
            Do not use emojis in your responses. Use plain text formatting only."#
            )
            .rmcp_tools(ripestat_conn.tools, ripestat_conn.peer)
            .rmcp_tools(whois_conn.tools, whois_conn.peer)
            .build();

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

        let res = agent.prompt(prompt).multi_turn(3).await?;

        Ok(res)
    }
}
