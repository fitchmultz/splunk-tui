//! Splunk TUI - Terminal user interface for Splunk Enterprise.
//!
//! Interactive terminal interface for managing Splunk deployments and running searches.

mod app;
mod event;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use app::{App, AppState, CurrentScreen};
use event::Event;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer())
        .init();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Run app
    let res = run_app(&mut terminal, &mut app).await;

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

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| app.render(f))?;

        // Handle events with timeout
        let timeout = tokio::time::Duration::from_millis(100);

        let event = if let Ok(event) = crossterm::event::poll(timeout) {
            if event {
                if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                    Event::Input(key)
                } else {
                    continue;
                }
            } else {
                Event::Tick
            }
        } else {
            Event::Tick
        };

        // Handle event
        if let Event::Input(key) = event {
            match app.state {
                AppState::Running => match app.current_screen {
                    CurrentScreen::Search => match key.code {
                        crossterm::event::KeyCode::Char('q') => {
                            return Ok(());
                        }
                        crossterm::event::KeyCode::Char('1') => {
                            app.current_screen = CurrentScreen::Search;
                        }
                        crossterm::event::KeyCode::Char('2') => {
                            app.current_screen = CurrentScreen::Indexes;
                        }
                        crossterm::event::KeyCode::Char('3') => {
                            app.current_screen = CurrentScreen::Cluster;
                        }
                        crossterm::event::KeyCode::Enter => {
                            if !app.search_input.is_empty() {
                                app.search_status = format!("Running: {}", app.search_input);
                            }
                        }
                        crossterm::event::KeyCode::Char(c) => {
                            app.search_input.push(c);
                        }
                        crossterm::event::KeyCode::Backspace => {
                            app.search_input.pop();
                        }
                        _ => {}
                    },
                    CurrentScreen::Indexes => match key.code {
                        crossterm::event::KeyCode::Char('q') => {
                            return Ok(());
                        }
                        crossterm::event::KeyCode::Char('1') => {
                            app.current_screen = CurrentScreen::Search;
                        }
                        crossterm::event::KeyCode::Char('2') => {
                            app.current_screen = CurrentScreen::Indexes;
                        }
                        crossterm::event::KeyCode::Char('3') => {
                            app.current_screen = CurrentScreen::Cluster;
                        }
                        _ => {}
                    },
                    CurrentScreen::Cluster => match key.code {
                        crossterm::event::KeyCode::Char('q') => {
                            return Ok(());
                        }
                        crossterm::event::KeyCode::Char('1') => {
                            app.current_screen = CurrentScreen::Search;
                        }
                        crossterm::event::KeyCode::Char('2') => {
                            app.current_screen = CurrentScreen::Indexes;
                        }
                        crossterm::event::KeyCode::Char('3') => {
                            app.current_screen = CurrentScreen::Cluster;
                        }
                        _ => {}
                    },
                },
                AppState::Quitting => {
                    return Ok(());
                }
            }
        }
    }
}
