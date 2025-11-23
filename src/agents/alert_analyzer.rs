use crate::http::server::BGPAlerterAlert;
use crate::mcp_clients;
use color_eyre::Result;
use rig::completion::Prompt;
use rig::prelude::CompletionClient;
use rig::providers::anthropic;

pub struct AlertAnalyzer;

impl AlertAnalyzer {
    pub async fn run(alert: BGPAlerterAlert, config: &crate::config::AppConfig) -> Result<String> {
        dotenv::dotenv().ok();

        tracing::info!("Starting hijack agent run");

        // Connect to RIPEstat MCP server
        let ripestat_conn = mcp_clients::connect_ripestat().await?;

        // Connect to WHOIS MCP server via stdio
        let whois_conn = mcp_clients::connect_whois().await?;

        let completion_model = anthropic::Client::from_env();

        let agent = completion_model
            .agent(&config.llm_model_name)
            .preamble(
                r#"
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

Your reports should take 30 seconds to read and act upon, not 5 minutes."#,
            )
            .rmcp_tools(ripestat_conn.tools, ripestat_conn.peer)
            .rmcp_tools(whois_conn.tools, whois_conn.peer)
            .build();

        let alert_json = serde_json::to_string_pretty(&alert)?;
        let res = agent
            .prompt(format!(
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
            ))
            .multi_turn(3)
            .await?;

        Ok(res)
    }
}
