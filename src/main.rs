mod agents;
mod alerts;
mod config;
mod database;
mod mcp_clients;
mod native_mcps;

use alerts::http;
use std::fs::OpenOptions;
use std::sync::Arc;
use tokio::sync::broadcast;

use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs")?;

    // Initialize tracing subscriber to write to a file
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/agent_noc.log")?;

    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Load configuration
    let config = config::AppConfig::from_env()?;
    let server_port = config.server_port;
    let server_url = format!("http://127.0.0.1:{server_port}");

    // Create broadcast channel for message streaming
    let (tx, _) = broadcast::channel::<String>(100);

    // Spawn server task
    let config_arc = Arc::new(config);
    let server_handle =
        tokio::spawn(async move { http::server::start(tx, Arc::clone(&config_arc)).await });

    // Wait a moment for server to start, then open browser
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Open browser
    if let Err(e) = open::that(&server_url) {
        tracing::warn!("Failed to open browser: {}", e);
        eprintln!("Failed to open browser: {e}. Please open {server_url} manually.");
    } else {
        tracing::info!("Opened browser at {}", server_url);
        println!("Opened browser at {server_url}");
    }

    // Keep server running
    match server_handle.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            tracing::error!("Server error: {}", e);
            Err(e)
        }
        Err(e) => {
            tracing::error!("Server task panicked: {}", e);
            Err(color_eyre::eyre::eyre!("Server task panicked: {}", e))
        }
    }
}
