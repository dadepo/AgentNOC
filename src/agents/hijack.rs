use crate::http::server::BGPAlerterAlert;
use crate::mcp_clients;
use color_eyre::Result;
use rig::completion::Prompt;
use rig::prelude::CompletionClient;
use rig::providers::anthropic;

pub struct HijackAgent;

impl HijackAgent {
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
            .preamble(r#"
            You are a BGP security analyst assisting NOC operators with analyzing potential BGP hijacking incidents.
            You have access to tools to gather information about IP prefixes, ASNs, and routing announcements. 
            Your role is to analyze BGP alerts, correlate them with known network configurations, and provide clear, actionable incident reports."#
        )
            .rmcp_tools(ripestat_conn.tools, ripestat_conn.peer)
            .rmcp_tools(whois_conn.tools, whois_conn.peer)
            .build();

        let alert_json = serde_json::to_string_pretty(&alert)?;
        let res = agent
            .prompt(format!(
                r#"Analyze the following BGP alert and prepare a comprehensive incident report for the NOC operator:

BGP Alert:
{alert_json}

Instructions:
1. Identify the key details from the alert: affected prefix, new prefix (if any), origin ASN changes, and peer information
2. Use whois tools to gather information about:
   - The affected prefix and its legitimate owner
   - The new origin ASN (if different from expected)
3. Assess the severity: Is this a potential hijack, a legitimate route change, or a configuration issue?
4. Provide a clear summary with:
   - Incident type and severity
   - Affected resources (prefixes, ASNs)
   - Legitimate vs. observed routing information
   - Recommended actions for the NOC operator
   - Any relevant historical context or patterns

Format your response as a structured incident report that is easy to scan and act upon."#
            ))
            .multi_turn(5)
            .await?;

        Ok(res)
    }
}
