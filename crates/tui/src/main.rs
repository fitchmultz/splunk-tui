//! Splunk TUI - Terminal user interface for Splunk Enterprise.
//!
//! Interactive terminal interface for managing Splunk deployments and running searches.

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use splunk_tui::action::Action;
use splunk_tui::app::App;
use splunk_tui::ui::ToastLevel;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::unbounded_channel};
use tracing_appender::non_blocking;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use splunk_client::{SplunkClient, models::HealthCheckOutput};
use splunk_config::{AuthStrategy as ConfigAuthStrategy, Config, ConfigLoader, ConfigManager};

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
                Ok(event) => match event {
                    crossterm::event::Event::Key(key) => {
                        if key.kind == crossterm::event::KeyEventKind::Press {
                            tx_input.send(Action::Input(key)).ok();
                        }
                    }
                    crossterm::event::Event::Mouse(mouse) => {
                        tx_input.send(Action::Mouse(mouse)).ok();
                    }
                    _ => {}
                },
                Err(_) => {
                    // Stream error, exit loop
                    break;
                }
            }
        }
    });

    // Load persisted configuration
    let config_manager = if let Ok(config_path) = std::env::var("SPLUNK_CONFIG_PATH") {
        if !config_path.is_empty() {
            ConfigManager::new_with_path(std::path::PathBuf::from(config_path))?
        } else {
            ConfigManager::new()?
        }
    } else {
        ConfigManager::new()?
    };
    let persisted_state = config_manager.load();
    let config_manager = Arc::new(Mutex::new(config_manager));

    // Create app with persisted state
    let mut app = App::new(Some(persisted_state));

    // Spawn background health monitoring task (60-second interval)
    let tx_health = tx.clone();
    let client_health = client.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let mut c = client_health.lock().await;
            match c.get_health().await {
                Ok(health) => {
                    if tx_health
                        .send(Action::HealthStatusLoaded(Ok(health)))
                        .is_err()
                    {
                        // Channel closed, exit task
                        break;
                    }
                }
                Err(e) => {
                    if tx_health
                        .send(Action::HealthStatusLoaded(Err(e.to_string())))
                        .is_err()
                    {
                        // Channel closed, exit task
                        break;
                    }
                }
            }
        }
    });

    // Create UI tick interval for smooth animations (250ms)
    let mut tick_interval = tokio::time::interval(tokio::time::Duration::from_millis(250));

    // Create data refresh interval (5 seconds, decoupled from UI tick)
    let mut refresh_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

    // Main event loop
    loop {
        terminal.draw(|f| app.render(f))?;

        tokio::select! {
            Some(action) = rx.recv() => {
                tracing::info!("Handling action: {:?}", splunk_tui::action::RedactedAction(&action));

                // Check for quit first
                if matches!(action, Action::Quit) {
                    if let Err(e) = save_and_quit(&app, &config_manager).await {
                        tracing::error!(error = %e, "Failed to save config");
                    }
                    break;
                }

                // Handle input -> Action
                if let Action::Input(key) = action {
                    if let Some(a) = app.handle_input(key) {
                        // Check for quit immediately after input handling
                        if matches!(a, Action::Quit) {
                            if let Err(e) = save_and_quit(&app, &config_manager).await {
                                tracing::error!(error = %e, "Failed to save config");
                            }
                            break;
                        }
                        let is_navigation = matches!(a, Action::NextScreen | Action::PreviousScreen);
                        app.update(a.clone());
                        handle_side_effects(a, client.clone(), tx.clone(), config_manager.clone()).await;
                        // If navigation action, trigger load for new screen
                        if is_navigation
                            && let Some(load_action) = app.load_action_for_screen()
                        {
                            handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone()).await;
                        }
                        // Check if we need to load more results after navigation
                        if let Some(load_action) = app.maybe_fetch_more_results() {
                            handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone()).await;
                        }
                    }
                } else if let Action::Mouse(mouse) = action {
                    if let Some(a) = app.handle_mouse(mouse) {
                        // Check for quit immediately after mouse handling
                        if matches!(a, Action::Quit) {
                            if let Err(e) = save_and_quit(&app, &config_manager).await {
                                tracing::error!(error = %e, "Failed to save config");
                            }
                            break;
                        }
                        let is_navigation = matches!(a, Action::NextScreen | Action::PreviousScreen);
                        app.update(a.clone());
                        handle_side_effects(a, client.clone(), tx.clone(), config_manager.clone()).await;
                        // If navigation action, trigger load for new screen
                        if is_navigation
                            && let Some(load_action) = app.load_action_for_screen()
                        {
                            handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone()).await;
                        }
                        // Check if we need to load more results after navigation
                        if let Some(load_action) = app.maybe_fetch_more_results() {
                            handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone()).await;
                        }
                    }
                } else {
                    app.update(action.clone());
                    handle_side_effects(action, client.clone(), tx.clone(), config_manager.clone()).await;
                }
            }
            _ = tick_interval.tick() => {
                // Always process tick for TTL pruning and animations
                app.update(Action::Tick);
            }
            _ = refresh_interval.tick() => {
                // Data refresh is separate from UI tick
                if let Some(a) = app.handle_tick() {
                    app.update(a.clone());
                    handle_side_effects(a, client.clone(), tx.clone(), config_manager.clone()).await;
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

/// Load configuration from environment and profile.
fn load_config() -> Result<Config> {
    let mut loader = ConfigLoader::new().load_dotenv()?;

    // Check for SPLUNK_CONFIG_PATH override (empty string is ignored)
    if let Ok(config_path) = std::env::var("SPLUNK_CONFIG_PATH")
        && !config_path.is_empty()
    {
        loader = loader.with_config_path(std::path::PathBuf::from(config_path));
    }

    // Load from profile if SPLUNK_PROFILE is set
    if let Ok(profile_name) = std::env::var("SPLUNK_PROFILE") {
        loader = loader.with_profile_name(profile_name).from_profile()?;
    }

    loader
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
    config_manager: Arc<Mutex<ConfigManager>>,
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
        Action::LoadSavedSearches => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_saved_searches().await {
                    Ok(searches) => {
                        tx.send(Action::SavedSearchesLoaded(Ok(searches))).ok();
                    }
                    Err(e) => {
                        tx.send(Action::SavedSearchesLoaded(Err(e.to_string())))
                            .ok();
                    }
                }
            });
        }
        Action::LoadInternalLogs => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                // Default to last 15 minutes of logs, 100 entries
                match c.get_internal_logs(100, Some("-15m")).await {
                    Ok(logs) => {
                        tx.send(Action::InternalLogsLoaded(Ok(logs))).ok();
                    }
                    Err(e) => {
                        tx.send(Action::InternalLogsLoaded(Err(e.to_string()))).ok();
                    }
                }
            });
        }
        Action::LoadApps => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_apps(None, None).await {
                    Ok(apps) => {
                        tx.send(Action::AppsLoaded(Ok(apps))).ok();
                    }
                    Err(e) => {
                        tx.send(Action::AppsLoaded(Err(e.to_string()))).ok();
                    }
                }
            });
        }
        Action::LoadUsers => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_users(None, None).await {
                    Ok(users) => {
                        tx.send(Action::UsersLoaded(Ok(users))).ok();
                    }
                    Err(e) => {
                        tx.send(Action::UsersLoaded(Err(e.to_string()))).ok();
                    }
                }
            });
        }
        Action::SwitchToSettings => {
            tx.send(Action::Loading(true)).ok();
            let config_manager_clone = config_manager.clone();
            tokio::spawn(async move {
                let cm = config_manager_clone.lock().await;
                let state = cm.load();
                tx.send(Action::SettingsLoaded(state)).ok();
            });
        }
        Action::RunSearch(query) => {
            tx.send(Action::Loading(true)).ok();
            tx.send(Action::Progress(0.1)).ok();

            let tx_clone = tx.clone();
            let query_clone = query.clone();
            tokio::spawn(async move {
                let mut c = client.lock().await;

                // Create search job
                let sid = match c.create_search_job(&query_clone, &Default::default()).await {
                    Ok(s) => s,
                    Err(e) => {
                        let mut details =
                            splunk_tui::error_details::ErrorDetails::from_client_error(&e);
                        details.add_context("query".to_string(), query_clone);
                        details
                            .add_context("operation".to_string(), "create_search_job".to_string());
                        tx_clone
                            .send(Action::ShowErrorDetails(details.clone()))
                            .ok();
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
                                    .send(Action::SearchComplete(Ok((
                                        results.results,
                                        sid,
                                        results.total,
                                    ))))
                                    .ok();
                            }
                            Err(e) => {
                                let mut details =
                                    splunk_tui::error_details::ErrorDetails::from_client_error(&e);
                                details.add_context("query".to_string(), query_clone);
                                details.add_context("sid".to_string(), sid.clone());
                                details.add_context(
                                    "operation".to_string(),
                                    "get_search_results".to_string(),
                                );
                                tx_clone
                                    .send(Action::ShowErrorDetails(details.clone()))
                                    .ok();
                                tx_clone
                                    .send(Action::SearchComplete(Err(e.to_string())))
                                    .ok();
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        let mut details =
                            splunk_tui::error_details::ErrorDetails::from_error_string(
                                &e.to_string(),
                            );
                        details.add_context("query".to_string(), query_clone);
                        details.add_context("sid".to_string(), sid.clone());
                        details.add_context("operation".to_string(), "wait_for_job".to_string());
                        tx_clone
                            .send(Action::ShowErrorDetails(details.clone()))
                            .ok();
                        tx_clone
                            .send(Action::SearchComplete(Err(e.to_string())))
                            .ok();
                    }
                    Err(_) => {
                        let mut details =
                            splunk_tui::error_details::ErrorDetails::from_error_string(
                                "Search timeout",
                            );
                        details.add_context("query".to_string(), query_clone);
                        details.add_context("sid".to_string(), sid.clone());
                        details.add_context("operation".to_string(), "wait_for_job".to_string());
                        tx_clone
                            .send(Action::ShowErrorDetails(details.clone()))
                            .ok();
                        tx_clone
                            .send(Action::SearchComplete(Err("Search timeout".to_string())))
                            .ok();
                    }
                }
            });
        }
        Action::LoadMoreSearchResults { sid, offset, count } => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.get_search_results(&sid, count, offset).await {
                    Ok(results) => {
                        tx.send(Action::MoreSearchResultsLoaded(Ok((
                            results.results,
                            offset,
                            results.total,
                        ))))
                        .ok();
                    }
                    Err(e) => {
                        tx.send(Action::MoreSearchResultsLoaded(Err(e.to_string())))
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
                        tx.send(Action::Notify(
                            ToastLevel::Error,
                            format!("Failed to cancel job: {}", e),
                        ))
                        .ok();
                        tx.send(Action::Loading(false)).ok();
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
                        tx.send(Action::Notify(
                            ToastLevel::Error,
                            format!("Failed to delete job: {}", e),
                        ))
                        .ok();
                        tx.send(Action::Loading(false)).ok();
                    }
                }
            });
        }
        Action::CancelJobsBatch(sids) => {
            tx.send(Action::Loading(true)).ok();
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                let mut success_count = 0;
                let mut error_messages = Vec::new();

                for sid in sids {
                    match c.cancel_job(&sid).await {
                        Ok(_) => {
                            success_count += 1;
                        }
                        Err(e) => {
                            error_messages.push(format!("{}: {}", sid, e));
                        }
                    }
                }

                let msg = if success_count > 0 {
                    format!("Cancelled {} job(s)", success_count)
                } else {
                    "No jobs cancelled".to_string()
                };

                if !error_messages.is_empty() {
                    for err in error_messages {
                        tx_clone.send(Action::Notify(ToastLevel::Error, err)).ok();
                    }
                }

                tx_clone.send(Action::JobOperationComplete(msg)).ok();
                tx_clone.send(Action::LoadJobs).ok();
            });
        }
        Action::DeleteJobsBatch(sids) => {
            tx.send(Action::Loading(true)).ok();
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                let mut success_count = 0;
                let mut error_messages = Vec::new();

                for sid in sids {
                    match c.delete_job(&sid).await {
                        Ok(_) => {
                            success_count += 1;
                        }
                        Err(e) => {
                            error_messages.push(format!("{}: {}", sid, e));
                        }
                    }
                }

                let msg = if success_count > 0 {
                    format!("Deleted {} job(s)", success_count)
                } else {
                    "No jobs deleted".to_string()
                };

                if !error_messages.is_empty() {
                    for err in error_messages {
                        tx_clone.send(Action::Notify(ToastLevel::Error, err)).ok();
                    }
                }

                tx_clone.send(Action::JobOperationComplete(msg)).ok();
                tx_clone.send(Action::LoadJobs).ok();
            });
        }
        Action::LoadHealth => {
            tx.send(Action::Loading(true)).ok();
            tokio::spawn(async move {
                let mut c = client.lock().await;

                // Construct the HealthCheckOutput
                let mut health_output = HealthCheckOutput {
                    server_info: None,
                    splunkd_health: None,
                    license_usage: None,
                    kvstore_status: None,
                    log_parsing_health: None,
                };

                let mut has_error = false;
                let mut error_messages = Vec::new();

                // Collect health info sequentially (due to &mut self requirement)
                match c.get_server_info().await {
                    Ok(info) => health_output.server_info = Some(info),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("Server info: {}", e));
                    }
                }

                match c.get_health().await {
                    Ok(health) => health_output.splunkd_health = Some(health),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("Splunkd health: {}", e));
                    }
                }

                match c.get_license_usage().await {
                    Ok(license) => health_output.license_usage = Some(license),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("License usage: {}", e));
                    }
                }

                match c.get_kvstore_status().await {
                    Ok(kvstore) => health_output.kvstore_status = Some(kvstore),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("KVStore status: {}", e));
                    }
                }

                match c.check_log_parsing_health().await {
                    Ok(log_parsing) => health_output.log_parsing_health = Some(log_parsing),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("Log parsing health: {}", e));
                    }
                }

                if has_error {
                    let combined_error = error_messages.join("; ");
                    tx.send(Action::HealthLoaded(Box::new(Err(combined_error))))
                        .ok();
                } else {
                    tx.send(Action::HealthLoaded(Box::new(Ok(health_output))))
                        .ok();
                }
            });
        }
        Action::ExportData(data, path, format) => {
            tokio::spawn(async move {
                let result = splunk_tui::export::export_value(&data, &path, format);

                match result {
                    Ok(_) => {
                        tx.send(Action::Notify(
                            ToastLevel::Info,
                            format!("Exported to {}", path.display()),
                        ))
                        .ok();
                    }
                    Err(e) => {
                        tx.send(Action::Notify(
                            ToastLevel::Error,
                            format!("Export failed: {}", e),
                        ))
                        .ok();
                    }
                }
            });
        }
        _ => {}
    }
}

/// Save persisted state and prepare to quit.
///
/// This function should be called before exiting the event loop to ensure
/// user preferences and UI state are persisted to disk.
async fn save_and_quit(app: &App, config_manager: &Arc<Mutex<ConfigManager>>) -> Result<()> {
    let state = app.get_persisted_state();
    let mut cm = config_manager.lock().await;
    cm.save(&state)?;
    Ok(())
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
