use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

pub fn start(rx: UnboundedReceiver<String>) -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal, rx);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal, mut rx: UnboundedReceiver<String>) -> Result<()> {
    let mut messages: Vec<String> = vec!["Waiting for agent results...".to_string()];

    loop {
        // Check for new messages from the channel (non-blocking)
        while let Ok(msg) = rx.try_recv() {
            messages.push(msg);
            // Keep only last N messages if needed
            if messages.len() > 50 {
                messages.remove(0);
            }
        }

        // Render the current state
        terminal.draw(|frame| {
            render(frame, &messages);
        })?;

        // Check for keyboard events with a timeout (non-blocking)
        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn render(frame: &mut Frame, messages: &[String]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)])
        .split(frame.area());

    let text: Vec<_> = messages.iter().map(|m| m.as_str()).collect();

    let paragraph = Paragraph::new(text.join("\n")).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Agent Results"),
    );

    frame.render_widget(paragraph, chunks[0]);
}
