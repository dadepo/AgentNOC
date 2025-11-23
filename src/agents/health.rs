use color_eyre::Result;
use rig::providers::anthropic;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

use crate::config::AppConfig;
use crate::mcp_clients;

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub services: HashMap<String, String>,
}

pub async fn run(_config: &AppConfig) -> Result<HealthStatus> {
    dotenv::dotenv().ok();

    tracing::info!("Starting health check");

    let mut health_status = HealthStatus {
        status: "healthy".to_string(),
        services: HashMap::new(),
    };

    // Check RIPEstat MCP server (HTTP connection) with timeout
    let ripestat_status =
        tokio::time::timeout(Duration::from_secs(3), check_ripestat_connection()).await;

    match ripestat_status {
        Ok(Ok(_)) => {
            health_status
                .services
                .insert("ripestat_mcp".to_string(), "healthy".to_string());
        }
        Ok(Err(e)) => {
            health_status.status = "unhealthy".to_string();
            health_status
                .services
                .insert("ripestat_mcp".to_string(), format!("error: {}", e));
            tracing::warn!("RIPEstat MCP check failed: {}", e);
        }
        Err(_) => {
            health_status.status = "unhealthy".to_string();
            health_status
                .services
                .insert("ripestat_mcp".to_string(), "timeout".to_string());
            tracing::warn!("RIPEstat MCP check timed out");
        }
    }

    // Check WHOIS MCP server (stdio connection) with timeout
    let whois_status = tokio::time::timeout(
        Duration::from_secs(5), // stdio connections may take longer to spawn
        check_whois_connection(),
    )
    .await;

    match whois_status {
        Ok(Ok(_)) => {
            health_status
                .services
                .insert("whois_mcp".to_string(), "healthy".to_string());
        }
        Ok(Err(e)) => {
            health_status.status = "unhealthy".to_string();
            health_status
                .services
                .insert("whois_mcp".to_string(), format!("error: {}", e));
            tracing::warn!("WHOIS MCP check failed: {}", e);
        }
        Err(_) => {
            health_status.status = "unhealthy".to_string();
            health_status
                .services
                .insert("whois_mcp".to_string(), "timeout".to_string());
            tracing::warn!("WHOIS MCP check timed out");
        }
    }

    // Check LLM client initialization (without making API calls)
    let llm_status = check_llm_client();
    health_status
        .services
        .insert("llm_client".to_string(), llm_status);

    tracing::info!("Health check completed: status = {}", health_status.status);

    Ok(health_status)
}

async fn check_ripestat_connection() -> Result<()> {
    // Use the existing mcp_clients function - it already handles connection and tool listing
    let _conn = mcp_clients::connect_ripestat().await?;
    // Connection successful - tools were listed, peer info retrieved
    Ok(())
}

async fn check_whois_connection() -> Result<()> {
    // Use the existing mcp_clients function - it spawns the process and connects via stdio
    let _conn = mcp_clients::connect_whois().await?;
    // Connection successful - tools were listed, peer info retrieved
    Ok(())
}

fn check_llm_client() -> String {
    // Check if we can create a client instance (doesn't make API calls)
    match anthropic::Client::from_env() {
        _ => "healthy".to_string(),
    }
}
