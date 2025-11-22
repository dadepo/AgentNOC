mod agents;
mod alerts;
mod config;
mod mcp_clients;

use alerts::http;
use std::fs::OpenOptions;
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
        .open("logs/noc_agent.log")?;

    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Load configuration
    let config = config::AppConfig::from_env()?;
    let server_port = config.server_port;
    let server_url = format!("http://127.0.0.1:{}", server_port);

    // Create broadcast channel for message streaming
    let (tx, _) = broadcast::channel::<String>(100);

    // Spawn server task
    let config_clone = config.clone();
    let server_handle = tokio::spawn(async move { http::server::start(tx, config_clone).await });

    // Wait a moment for server to start, then open browser
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Open browser
    if let Err(e) = open::that(&server_url) {
        tracing::warn!("Failed to open browser: {}", e);
        eprintln!(
            "Failed to open browser: {}. Please open {} manually.",
            e, server_url
        );
    } else {
        tracing::info!("Opened browser at {}", server_url);
        println!("Opened browser at {}", server_url);
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
