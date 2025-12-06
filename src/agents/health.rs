use color_eyre::Result;
use rig::providers::anthropic;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::time::Duration;

use crate::config::AppConfig;
use crate::database::db::get_enabled_mcp_servers;
use crate::mcp_clients;

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub services: HashMap<String, String>,
}

pub async fn run(_config: &AppConfig, db_pool: &SqlitePool) -> Result<HealthStatus> {
    dotenv::dotenv().ok();

    tracing::info!("Starting health check");

    let mut health_status = HealthStatus {
        status: "healthy".to_string(),
        services: HashMap::new(),
    };

    // Get all enabled MCP servers and check each one
    match get_enabled_mcp_servers(db_pool).await {
        Ok(servers) => {
            if servers.is_empty() {
                health_status.services.insert(
                    "mcp_servers".to_string(),
                    "no servers configured".to_string(),
                );
            } else {
                for server in servers {
                    let server_name = format!("mcp_{}", server.name());

                    // Check connection with timeout
                    let check_result = tokio::time::timeout(
                        Duration::from_secs(5),
                        mcp_clients::test_connection(&server),
                    )
                    .await;

                    match check_result {
                        Ok(Ok(tool_count)) => {
                            health_status
                                .services
                                .insert(server_name, format!("healthy ({} tools)", tool_count));
                        }
                        Ok(Err(e)) => {
                            health_status.status = "degraded".to_string();
                            health_status
                                .services
                                .insert(server_name, format!("error: {e}"));
                            tracing::warn!("MCP server '{}' check failed: {}", server.name(), e);
                        }
                        Err(_) => {
                            health_status.status = "degraded".to_string();
                            health_status
                                .services
                                .insert(server_name, "timeout".to_string());
                            tracing::warn!("MCP server '{}' check timed out", server.name());
                        }
                    }
                }
            }
        }
        Err(e) => {
            health_status.status = "unhealthy".to_string();
            health_status
                .services
                .insert("mcp_servers".to_string(), format!("database error: {e}"));
            tracing::error!("Failed to get MCP servers for health check: {}", e);
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

fn check_llm_client() -> String {
    // Check if we can create a client instance (doesn't make API calls)
    anthropic::Client::from_env();
    "healthy".to_string()
}
