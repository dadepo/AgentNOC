use crate::alerts::http::server::BGPAlerterAlert;
use crate::config::ANTHROPIC_MAX_TOKENS;
use crate::mcp_clients::{self, MCPConnection};
use color_eyre::Result;
use rig::client::ProviderClient;
use rig::completion::Prompt;
use rig::prelude::CompletionClient;
use rig::providers::anthropic;
use sqlx::SqlitePool;

pub struct AlertAnalyzer;

const PREAMBLE: &str = r#"
You are a BGP security analyst for busy NOC operators who need FAST, ACTIONABLE insights.

CRITICAL: Use your available tools to PROACTIVELY gather enrichment data.
The operator should NOT need to run queries themselves - you do the lookups and include relevant 
context directly in your report (ownership info, ASN details, historical patterns, etc.).

COMMUNICATION RULES:
- Be extremely concise - every word must add value
- Lead with the most critical information
- Assume the operator understands BGP basics
- Focus on "what to do" over "what happened"
- No emojis, minimal formatting
- If tools fail, provide analysis based solely on alert data and mention tool failures briefly

Your reports should take 30 seconds to read and act upon, not 5 minutes."#;

impl AlertAnalyzer {
    pub async fn run(
        alert: BGPAlerterAlert,
        config: &crate::config::AppConfig,
        db_pool: &SqlitePool,
    ) -> Result<String> {
        dotenv::dotenv().ok();

        tracing::info!("Starting hijack agent run");

        // Connect to all enabled MCP servers from database
        let mcp_connections = mcp_clients::connect_all_enabled(db_pool).await?;

        if mcp_connections.is_empty() {
            tracing::warn!("No MCP servers available - agent will run without tools");
        } else {
            let total_tools: usize = mcp_connections.iter().map(|c| c.tool_count()).sum();
            tracing::info!(
                "Connected to {} MCP server(s) with {} total tool(s)",
                mcp_connections.len(),
                total_tools
            );
        }

        let completion_model = anthropic::Client::from_env();

        let alert_json = serde_json::to_string_pretty(&alert)?;
        let prompt = Self::build_prompt(&alert_json);

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

    fn build_prompt(alert_json: &str) -> String {
        format!(
            r#"Analyze this BGP alert and respond with ONLY a valid JSON object. NO markdown, NO explanations, JUST the JSON.

BGP Alert:
{alert_json}

CRITICAL INSTRUCTIONS:
1. USE YOUR TOOLS: Query WHOIS/RIPEstat for ASN ownership, prefix registration, and historical data
2. ENRICH YOUR RESPONSE: Include context the operator would need (who owns the ASNs, legitimacy indicators, etc.)
3. SAVE OPERATOR TIME: They should NOT need to run additional queries - you provide all relevant context
4. BE SPECIFIC: Include actual organization names, registration details, and concrete evidence in your assessment

Required JSON structure:
{{
  "summary": "2-3 sentence executive summary with enriched context (include ASN owner names, registration status, etc.)",
  "severity": "Critical|High|Medium|Low|Info",
  "key_facts": {{
    "affected_prefix": "prefix from alert with registration owner if found",
    "expected_asn": "expected ASN with organization name (e.g. 'AS3333 (RIPE NCC)')",
    "observed_asn": "observed ASN with organization name (e.g. 'AS9999 (Unknown Operator)')",
    "duration": "human readable duration from earliest to latest",
    "peer_count": number of peers (count from alert)
  }},
  "immediate_actions": [
    "First action with specific contact info or validation method if available",
    "Second action with concrete steps based on enrichment data",
    "Third action informed by historical patterns or registration info"
  ],
  "risk_assessment": "1-2 sentence analysis informed by tool lookups (ownership conflicts, legitimacy indicators, known relationships)",
  "tool_notes": "Brief summary of enrichment data gathered or any tool failures"
}}

EXAMPLES of enriched responses:
- Good: "AS9999 (Suspicious Networks Inc.) announcing prefix registered to AS3333 (RIPE NCC)"
- Bad: "AS9999 announcing prefix expected from AS3333"

CRITICAL: Output ONLY valid JSON. No markdown code blocks, no extra text."#
        )
    }

    async fn run_agent_with_tools(
        client: anthropic::Client,
        model_name: &str,
        connections: Vec<MCPConnection>,
        prompt: &str,
    ) -> Result<String> {
        // Handle the case with no MCP connections
        if connections.is_empty() {
            let agent = client
                .agent(model_name)
                .preamble(PREAMBLE)
                .max_tokens(ANTHROPIC_MAX_TOKENS)
                .build();
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
            .max_tokens(ANTHROPIC_MAX_TOKENS)
            .rmcp_tools(first_conn.tools, first_conn.peer);

        // Add remaining connections
        for conn in connections_iter {
            agent_builder = agent_builder.rmcp_tools(conn.tools, conn.peer);
        }

        let agent = agent_builder.build();
        Ok(agent.prompt(prompt).multi_turn(3).await?)
    }
}
