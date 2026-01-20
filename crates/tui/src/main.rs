//! Splunk TUI - Terminal user interface for Splunk Enterprise.
//!
//! Interactive terminal interface for managing Splunk deployments and running searches.

mod action;
mod app;
mod ui;

use action::Action;
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::unbounded_channel};
use tracing_appender::non_blocking;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use app::App;
use splunk_client::SplunkClient;
use splunk_config::{AuthStrategy as ConfigAuthStrategy, Config, ConfigLoader};

/// Shared client wrapper for async tasks.
type SharedClient = Arc<Mutex<SplunkClient>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs")?;

    // Initialize file-based logging
    let file_appender = tracing_appender::rolling::daily("logs", "splunk-tui.log");
    let (non_blocking, _guard) = non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().with_writer(non_blocking))
        .init();

    // Note: _guard must live for entire main() duration to ensure logs are flushed

    // Load config at startup
    let config = load_config()?;

    // Build and authenticate client
    let client = Arc::new(Mutex::new(create_client(&config).await?));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create channel for actions
    let (tx, mut rx) = unbounded_channel::<Action>();

    // Spawn input stream task
    let tx_input = tx.clone();
    tokio::spawn(async move {
        use crossterm::event::EventStream;

        let mut reader = EventStream::new();
        while let Some(event_result) = reader.next().await {
            match event_result {
                Ok(event) => {
                    if let crossterm::event::Event::Key(key) = event {
                        tx_input.send(Action::Input(key)).ok();
                    }
                    // Resize is handled automatically by ratatui on next draw
                }
                Err(_) => {
                    // Stream error, exit loop
                    break;
                }
            }
        }
    });

    // Create app
    let mut app = App::new();

    // Create auto-refresh interval (5 seconds)
    let mut refresh_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

    // Main event loop
    loop {
        terminal.draw(|f| app.render(f))?;

        tokio::select! {
            Some(action) = rx.recv() => {
                // Log action for observability
                tracing::info!(?action, "Handling action");

                // Check for quit first
                if matches!(action, Action::Quit) {
                    break;
                }

                // Handle input -> Action
                if let Action::Input(key) = action {
                    if let Some(a) = app.handle_input(key) {
                        app.update(a.clone());
                        handle_side_effects(a, client.clone(), tx.clone()).await;
                    }
                } else {
                    app.update(action.clone());
                    handle_side_effects(action, client.clone(), tx.clone()).await;
                }
            }
            _ = refresh_interval.tick() => {
                if let Some(a) = app.handle_tick() {
                    app.update(a.clone());
                    handle_side_effects(a, client.clone(), tx.clone()).await;
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// Load configuration from environment.
fn load_config() -> Result<Config> {
    ConfigLoader::new()
        .load_dotenv()?
        .from_env()?
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))
}

/// Create and authenticate a new Splunk client.
async fn create_client(config: &Config) -> Result<SplunkClient> {
    let auth_strategy = match &config.auth.strategy {
        ConfigAuthStrategy::SessionToken { username, password } => {
            splunk_client::AuthStrategy::SessionToken {
                username: username.clone(),
                password: password.clone(),
            }
        }
        ConfigAuthStrategy::ApiToken { token } => splunk_client::AuthStrategy::ApiToken {
            token: token.clone(),
        },
    };

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url.clone())
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    // Login if using session token
    if !client.is_api_token_auth() {
        client.login().await?;
    }

    Ok(client)
}

/// Handle side effects (async API calls) for actions.
async fn handle_side_effects(
    action: Action,
    client: SharedClient,
    tx: tokio::sync::mpsc::UnboundedSender<Action>,
) {
    match action {
        Action::LoadIndexes => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_indexes(None, None).await {
                    Ok(indexes) => {
                        tx.send(Action::IndexesLoaded(Ok(indexes))).ok();
                    }
                    Err(e) => {
                        tx.send(Action::IndexesLoaded(Err(e.to_string()))).ok();
                    }
                }
            });
        }
        Action::LoadJobs => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_jobs(None, None).await {
                    Ok(jobs) => {
                        tx.send(Action::JobsLoaded(Ok(jobs))).ok();
                    }
                    Err(e) => {
                        tx.send(Action::JobsLoaded(Err(e.to_string()))).ok();
                    }
                }
            });
        }
        Action::LoadClusterInfo => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.get_cluster_info().await {
                    Ok(info) => {
                        tx.send(Action::ClusterInfoLoaded(Ok(info))).ok();
                    }
                    Err(e) => {
                        tx.send(Action::ClusterInfoLoaded(Err(e.to_string()))).ok();
                    }
                }
            });
        }
        Action::RunSearch(query) => {
            tx.send(Action::Loading(true)).ok();
            tx.send(Action::Progress(0.1)).ok();

            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let mut c = client.lock().await;

                // Create the job
                let sid = match c.create_search_job(&query, &Default::default()).await {
                    Ok(s) => s,
                    Err(e) => {
                        tx_clone
                            .send(Action::SearchComplete(Err(e.to_string())))
                            .ok();
                        return;
                    }
                };

                tx_clone.send(Action::Progress(0.5)).ok();

                // Wait for completion (simplified - in production would poll)
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(300),
                    wait_for_job(&mut c, &sid, tx_clone.clone()),
                )
                .await
                {
                    Ok(Ok(())) => {
                        // Get results
                        match c.get_search_results(&sid, 1000, 0).await {
                            Ok(results) => {
                                tx_clone.send(Action::Progress(1.0)).ok();
                                tx_clone
                                    .send(Action::SearchComplete(Ok((results.results, sid))))
                                    .ok();
                            }
                            Err(e) => {
                                tx_clone
                                    .send(Action::SearchComplete(Err(e.to_string())))
                                    .ok();
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        tx_clone
                            .send(Action::SearchComplete(Err(e.to_string())))
                            .ok();
                    }
                    Err(_) => {
                        tx_clone
                            .send(Action::SearchComplete(Err("Search timeout".to_string())))
                            .ok();
                    }
                }
            });
        }
        Action::CancelJob(sid) => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.cancel_job(&sid).await {
                    Ok(_) => {
                        tx.send(Action::JobOperationComplete(format!(
                            "Cancelled job: {}",
                            sid
                        )))
                        .ok();
                        // Reload the job list
                        tx.send(Action::LoadJobs).ok();
                    }
                    Err(e) => {
                        tx.send(Action::Error(format!("Failed to cancel job: {}", e)))
                            .ok();
                    }
                }
            });
        }
        Action::DeleteJob(sid) => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.delete_job(&sid).await {
                    Ok(_) => {
                        tx.send(Action::JobOperationComplete(format!(
                            "Deleted job: {}",
                            sid
                        )))
                        .ok();
                        // Reload the job list
                        tx.send(Action::LoadJobs).ok();
                    }
                    Err(e) => {
                        tx.send(Action::Error(format!("Failed to delete job: {}", e)))
                            .ok();
                    }
                }
            });
        }
        _ => {}
    }
}

/// Wait for a job to complete by polling its status.
async fn wait_for_job(
    client: &mut SplunkClient,
    sid: &str,
    tx: tokio::sync::mpsc::UnboundedSender<Action>,
) -> Result<()> {
    use tokio::time::{Duration, sleep};

    loop {
        sleep(Duration::from_millis(500)).await;

        let status = client.get_job_status(sid).await?;

        // Update progress
        tx.send(Action::Progress(status.done_progress as f32)).ok();

        if status.is_done {
            return Ok(());
        }
    }
}
