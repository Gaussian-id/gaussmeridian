//! GaussMeridian TUI - Terminal User Interface
//!
//! A professional terminal interface for managing and monitoring GaussMeridian.

use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

mod app;
mod api;
mod error;
mod events;
mod state;
mod ui;

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Parse command line arguments for API URL and key
    let api_url = std::env::var("GAUSSMERIDIAN_API_URL")
        .ok();
    let api_key = std::env::var("GAUSSMERIDIAN_API_KEY")
        .ok();

    // Create and run app
    let mut app = App::new(api_url, api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create app: {}", e))?;
    let res = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

