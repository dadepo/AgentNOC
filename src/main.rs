mod agents;
mod alerts;
mod ui;

use alerts::http;
use ui::tui::terminal;

use std::thread;
use tokio::sync::mpsc;

use color_eyre::Result;

fn main() -> Result<()> {
    // Initialize tracing subscriber once at the start
    tracing_subscriber::fmt::init();

    let (tx, rx) = mpsc::unbounded_channel();

    // Spawn the alerts thread
    let handle = thread::spawn(move || -> Result<(), String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

        rt.block_on(http::server::start(tx))
            .map_err(|e| format!("Alert server error: {}", e))?;

        Ok(())
    });

    terminal::start(rx)?;

    match handle.join() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(color_eyre::eyre::eyre!("{}", e)),
        Err(e) => {
            return Err(color_eyre::eyre::eyre!(
                "Alert server thread panicked: {:?}",
                e
            ));
        }
    }

    Ok(())
}
