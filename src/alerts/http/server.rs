use crate::agents::health;
use axum::{Json, Router, http::StatusCode, routing::get};
use color_eyre::Result;
use tokio::sync::mpsc::UnboundedSender;

pub async fn start(tx: UnboundedSender<String>) -> Result<()> {
    // build our application with a route
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .with_state(tx);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7654").await?;
    tracing::info!("Server starting on http://0.0.0.0:7654");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> &'static str {
    "NOC Agent Server is running!"
}

async fn health(
    axum::extract::State(tx): axum::extract::State<UnboundedSender<String>>,
) -> Result<Json<String>, StatusCode> {
    match health::run("8.8.8.8".to_string()).await {
        Ok(result) => {
            // Send result to terminal
            let _ = tx.send(format!("Agent result: {}", result));
            Ok(Json(result))
        }
        Err(e) => {
            let _ = tx.send(format!("Agent error: {}", e));
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
