//! Splunk TUI - Terminal user interface for Splunk Enterprise.
//!
//! Responsibilities:
//! - Provide an interactive terminal interface for Splunk.
//! - Manage application state, UI rendering, and user input handling.
//! - Handle background tasks for health monitoring and data fetching.
//! - Parse command-line arguments for configuration overrides.
//!
//! Does NOT handle:
//! - Core business logic or REST API implementation (see `crates/client`).
//! - Manual configuration file editing (see `crates/cli`).
//! - Configuration persistence (see `crates/config`).
//!
//! Invariants / Assumptions:
//! - The TUI enters raw mode and alternate screen on startup.
//! - `load_dotenv()` is called at startup to support `.env` configuration.
//! - Configuration precedence: CLI args > env vars > profile config > defaults.
//! - Mouse capture is enabled by default unless `--no-mouse` is specified.

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use splunk_tui::action::{Action, progress_callback_to_action_sender};
use splunk_tui::app::{App, ConnectionContext};
use splunk_tui::error_details::{build_search_error_details, search_error_message};
use splunk_tui::ui::ToastLevel;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::channel};
use tracing_appender::non_blocking;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use splunk_client::{SplunkClient, models::HealthCheckOutput};
use splunk_config::{
    AuthStrategy as ConfigAuthStrategy, Config, ConfigLoader, ConfigManager, SearchDefaults,
    env_var_or_none,
};

/// Command-line arguments for splunk-tui.
///
/// Configuration precedence (highest to lowest):
/// 1. CLI arguments (e.g., --profile, --config-path)
/// 2. Environment variables (e.g., SPLUNK_PROFILE, SPLUNK_BASE_URL)
/// 3. Profile configuration (from config.json)
/// 4. Default values
#[derive(Debug, Parser)]
#[command(
    name = "splunk-tui",
    about = "Terminal user interface for Splunk Enterprise",
    version,
    after_help = "Examples:\n  splunk-tui\n  splunk-tui --profile production\n  splunk-tui --config-path /etc/splunk-tui/config.json\n  splunk-tui --log-dir /var/log/splunk-tui --no-mouse\n"
)]
struct Cli {
    /// Config profile name to load
    #[arg(long, short = 'p')]
    profile: Option<String>,

    /// Path to a custom configuration file
    #[arg(long)]
    config_path: Option<PathBuf>,

    /// Directory for log files
    #[arg(long, default_value = "logs")]
    log_dir: PathBuf,

    /// Disable mouse support
    #[arg(long)]
    no_mouse: bool,
}

/// Shared client wrapper for async tasks.
type SharedClient = Arc<Mutex<SplunkClient>>;

/// Guard that ensures terminal state is restored on drop.
///
/// This struct captures the terminal state configuration and restores
/// it when dropped, ensuring cleanup happens even during panics.
///
/// # Invariants
/// - Must be created after terminal setup is complete
/// - Must live for the duration of the TUI session
/// - Drop implementation must not panic
struct TerminalGuard {
    no_mouse: bool,
}

impl TerminalGuard {
    /// Create a new terminal guard.
    ///
    /// # Arguments
    /// * `no_mouse` - Whether mouse capture was disabled during setup
    fn new(no_mouse: bool) -> Self {
        Self { no_mouse }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Restore terminal state, ignoring errors since we're in drop
        // and must not panic. The explicit cleanup in main() runs first
        // on normal exit; this is a safety net for panics and signals.
        let _ = disable_raw_mode();
        let mut stdout = std::io::stdout();
        if self.no_mouse {
            let _ = execute!(stdout, LeaveAlternateScreen);
        } else {
            let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Capture no_mouse flag for later use in cleanup
    let no_mouse = cli.no_mouse;

    // Create logs directory if it doesn't exist
    std::fs::create_dir_all(&cli.log_dir)?;

    // Initialize file-based logging with configurable directory
    let log_file_name = "splunk-tui.log";
    let file_appender = tracing_appender::rolling::daily(&cli.log_dir, log_file_name);
    let (non_blocking, _guard) = non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().with_writer(non_blocking))
        .init();

    // Note: _guard must live for entire main() duration to ensure logs are flushed

    // Load config at startup with CLI overrides
    // Also get search defaults with env var overrides applied
    let (search_default_config, config) = load_config_with_search_defaults(&cli)?;

    // Build and authenticate client
    let client = Arc::new(Mutex::new(create_client(&config).await?));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();

    // Conditionally enable mouse capture based on CLI flag
    if no_mouse {
        execute!(stdout, EnterAlternateScreen)?;
    } else {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    }

    // Create guard to ensure terminal restoration on panic/unwind.
    // This ensures the terminal is restored even if the application panics
    // or receives a signal that causes unwinding.
    let _terminal_guard = TerminalGuard::new(no_mouse);

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create bounded channel for actions with backpressure handling
    // Capacity chosen to handle normal input bursts without blocking
    // while preventing unbounded growth under extreme load
    const ACTION_CHANNEL_CAPACITY: usize = 256;
    let (tx, mut rx) = channel::<Action>(ACTION_CHANNEL_CAPACITY);

    // Spawn input stream task with backpressure handling
    let tx_input = tx.clone();
    tokio::spawn(async move {
        use crossterm::event::EventStream;
        use tokio::sync::mpsc::error::TrySendError;

        let mut reader = EventStream::new();
        while let Some(event_result) = reader.next().await {
            match event_result {
                Ok(event) => {
                    let action = match event {
                        crossterm::event::Event::Key(key) => {
                            if key.kind == crossterm::event::KeyEventKind::Press {
                                Some(Action::Input(key))
                            } else {
                                None
                            }
                        }
                        crossterm::event::Event::Mouse(mouse) => Some(Action::Mouse(mouse)),
                        _ => None,
                    };

                    if let Some(action) = action {
                        match tx_input.try_send(action) {
                            Ok(()) => {}
                            Err(TrySendError::Full(_)) => {
                                // Channel full - drop input event (backpressure)
                                // This is acceptable for input events as they're
                                // time-sensitive; old input is less valuable than new
                                tracing::debug!("Input channel full, dropping input event");
                            }
                            Err(TrySendError::Closed(_)) => {
                                // Channel closed, exit task
                                break;
                            }
                        }
                    }
                }
                Err(_) => {
                    // Stream error, exit loop
                    break;
                }
            }
        }
    });

    // Load persisted configuration
    // CLI --config-path takes precedence over SPLUNK_CONFIG_PATH env var
    let config_manager = if let Some(config_path) = &cli.config_path {
        ConfigManager::new_with_path(config_path.clone())?
    } else if let Some(config_path) = env_var_or_none("SPLUNK_CONFIG_PATH") {
        ConfigManager::new_with_path(std::path::PathBuf::from(config_path))?
    } else {
        ConfigManager::new()?
    };
    let mut persisted_state = config_manager.load();
    let config_manager = Arc::new(Mutex::new(config_manager));

    // Apply environment variable overrides to search defaults
    // Precedence: env vars > persisted values > hardcoded defaults
    persisted_state.search_defaults = SearchDefaults {
        earliest_time: search_default_config.earliest_time,
        latest_time: search_default_config.latest_time,
        max_results: search_default_config.max_results,
    };

    // Initialize keybinding overrides from persisted state
    if let Err(e) =
        splunk_tui::input::keymap::overrides::init_overrides(&persisted_state.keybind_overrides)
    {
        tracing::warn!(
            "Failed to initialize keybinding overrides: {}. Using defaults.",
            e
        );
    }

    // Build connection context for TUI header display (RQ-0134)
    let connection_ctx = ConnectionContext {
        profile_name: cli
            .profile
            .clone()
            .or_else(|| env_var_or_none("SPLUNK_PROFILE")),
        base_url: config.connection.base_url.clone(),
        auth_mode: match &config.auth.strategy {
            ConfigAuthStrategy::ApiToken { .. } => "token".to_string(),
            ConfigAuthStrategy::SessionToken { username, .. } => {
                format!("session ({username})")
            }
        },
    };

    // Create app with persisted state (now includes env var overrides for search defaults)
    let mut app = App::new(Some(persisted_state), connection_ctx);

    // Spawn background health monitoring task (configurable interval, default 60s)
    let tx_health = tx.clone();
    let client_health = client.clone();
    let health_check_interval = config.connection.health_check_interval_seconds;
    tokio::spawn(async move {
        use tokio::sync::mpsc::error::TrySendError;

        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(health_check_interval));
        loop {
            interval.tick().await;
            let mut c = client_health.lock().await;
            match c.get_health().await {
                Ok(health) => {
                    match tx_health.try_send(Action::HealthStatusLoaded(Ok(health))) {
                        Ok(()) => {}
                        Err(TrySendError::Full(_)) => {
                            // Drop health status update if channel full - next tick will send another
                            tracing::debug!("Health status channel full, dropping update");
                        }
                        Err(TrySendError::Closed(_)) => {
                            // Channel closed, exit task
                            break;
                        }
                    }
                }
                Err(e) => {
                    match tx_health.try_send(Action::HealthStatusLoaded(Err(e.to_string()))) {
                        Ok(()) => {}
                        Err(TrySendError::Full(_)) => {
                            // Drop health status update if channel full - next tick will send another
                            tracing::debug!("Health status channel full, dropping update");
                        }
                        Err(TrySendError::Closed(_)) => {
                            // Channel closed, exit task
                            break;
                        }
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
                    let was_toggle = matches!(action, Action::ToggleClusterViewMode);
                    let was_profile_switch = matches!(action, Action::ProfileSwitchResult(Ok(_)));
                    app.update(action.clone());
                    handle_side_effects(action, client.clone(), tx.clone(), config_manager.clone()).await;
                    // After toggle, if we're now in Peers view, trigger peers load
                    if was_toggle && app.cluster_view_mode == splunk_tui::app::ClusterViewMode::Peers {
                        handle_side_effects(Action::LoadClusterPeers, client.clone(), tx.clone(), config_manager.clone()).await;
                    }
                    // After successful profile switch, trigger reload for current screen
                    if was_profile_switch
                        && let Some(load_action) = app.load_action_for_screen()
                    {
                        handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone()).await;
                    }
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

    // Conditionally disable mouse capture based on CLI flag
    if no_mouse {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    } else {
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
    }

    terminal.show_cursor()?;

    Ok(())
}

/// Load configuration and search defaults from CLI args, environment variables, and profile.
///
/// This function returns both the main Config and the SearchDefaultConfig so that
/// search defaults with environment variable overrides can be applied to the App state.
///
/// Configuration precedence (highest to lowest):
/// 1. CLI arguments (e.g., --profile, --config-path)
/// 2. Environment variables (e.g., SPLUNK_PROFILE, SPLUNK_BASE_URL)
/// 3. Profile configuration (from config.json)
/// 4. Default values
///
/// # Arguments
///
/// * `cli` - The parsed CLI arguments
///
/// # Errors
///
/// Returns an error if configuration loading fails (e.g., profile not found,
/// missing required fields like base_url or auth credentials).
fn load_config_with_search_defaults(
    cli: &Cli,
) -> Result<(splunk_config::SearchDefaultConfig, Config)> {
    let mut loader = ConfigLoader::new().load_dotenv()?;

    // Apply config path from CLI if provided (highest precedence)
    if let Some(config_path) = &cli.config_path {
        loader = loader.with_config_path(config_path.clone());
    } else if let Some(config_path) = env_var_or_none("SPLUNK_CONFIG_PATH") {
        // Fall back to env var
        loader = loader.with_config_path(std::path::PathBuf::from(config_path));
    }

    // Load from profile if specified via CLI or env
    // CLI --profile takes precedence over SPLUNK_PROFILE env var
    let profile_name = cli
        .profile
        .clone()
        .or_else(|| env_var_or_none("SPLUNK_PROFILE"));

    if let Some(profile) = profile_name {
        loader = loader.with_profile_name(profile).from_profile()?;
    }

    // Environment variables are loaded last - they override profile values
    let loader = loader.from_env()?;

    // Build search defaults with env var overrides (pass None for now, will merge with persisted later)
    let search_defaults = loader.build_search_defaults(None);

    let config = loader
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    Ok((search_defaults, config))
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
        .session_ttl_seconds(config.connection.session_ttl_seconds)
        .session_expiry_buffer_seconds(config.connection.session_expiry_buffer_seconds)
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
    tx: tokio::sync::mpsc::Sender<Action>,
    config_manager: Arc<Mutex<ConfigManager>>,
) {
    match action {
        Action::LoadIndexes => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_indexes(None, None).await {
                    Ok(indexes) => {
                        let _ = tx.send(Action::IndexesLoaded(Ok(indexes))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::IndexesLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::LoadJobs => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_jobs(None, None).await {
                    Ok(jobs) => {
                        let _ = tx.send(Action::JobsLoaded(Ok(jobs))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::JobsLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::LoadClusterInfo => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.get_cluster_info().await {
                    Ok(info) => {
                        let _ = tx.send(Action::ClusterInfoLoaded(Ok(info))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ClusterInfoLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::LoadClusterPeers => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.get_cluster_peers().await {
                    Ok(peers) => {
                        let _ = tx.send(Action::ClusterPeersLoaded(Ok(peers))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::ClusterPeersLoaded(Err(e.to_string())))
                            .await;
                    }
                }
            });
        }
        Action::LoadSavedSearches => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_saved_searches().await {
                    Ok(searches) => {
                        let _ = tx.send(Action::SavedSearchesLoaded(Ok(searches))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::SavedSearchesLoaded(Err(e.to_string())))
                            .await;
                    }
                }
            });
        }
        Action::LoadInternalLogs => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                // Default to last 15 minutes of logs, 100 entries
                match c.get_internal_logs(100, Some("-15m")).await {
                    Ok(logs) => {
                        let _ = tx.send(Action::InternalLogsLoaded(Ok(logs))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::InternalLogsLoaded(Err(e.to_string())))
                            .await;
                    }
                }
            });
        }
        Action::LoadApps => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_apps(None, None).await {
                    Ok(apps) => {
                        let _ = tx.send(Action::AppsLoaded(Ok(apps))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::AppsLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::LoadUsers => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_users(None, None).await {
                    Ok(users) => {
                        let _ = tx.send(Action::UsersLoaded(Ok(users))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::UsersLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::SwitchToSettings => {
            let _ = tx.send(Action::Loading(true)).await;
            let config_manager_clone = config_manager.clone();
            tokio::spawn(async move {
                let cm = config_manager_clone.lock().await;
                let state = cm.load();
                let _ = tx.send(Action::SettingsLoaded(state)).await;
            });
        }
        Action::RunSearch {
            query,
            search_defaults,
        } => {
            let _ = tx.send(Action::Loading(true)).await;
            let _ = tx.send(Action::Progress(0.1)).await;

            // Store the query that is about to run for accurate status messages
            let _ = tx.send(Action::SearchStarted(query.clone())).await;

            let tx_clone = tx.clone();
            let query_clone = query.clone();
            tokio::spawn(async move {
                let mut c = client.lock().await;

                // Create progress callback that sends Action::Progress via channel
                let progress_tx = tx_clone.clone();
                let mut progress_callback = progress_callback_to_action_sender(progress_tx);

                // Use search_with_progress for unified timeout and progress handling
                match c
                    .search_with_progress(
                        &query_clone,
                        true, // wait for completion
                        Some(&search_defaults.earliest_time),
                        Some(&search_defaults.latest_time),
                        Some(search_defaults.max_results),
                        Some(&mut progress_callback),
                    )
                    .await
                {
                    Ok((results, sid, total)) => {
                        let _ = tx_clone.send(Action::Progress(1.0)).await;
                        let _ = tx_clone
                            .send(Action::SearchComplete(Ok((results, sid, total))))
                            .await;
                    }
                    Err(e) => {
                        let details = build_search_error_details(
                            &e,
                            query_clone,
                            "search_with_progress".to_string(),
                            None, // SID not available on failure
                        );
                        let error_msg = search_error_message(&e);
                        // Error details stored in SearchComplete handler; user can press 'e' to view
                        let _ = tx_clone
                            .send(Action::SearchComplete(Err((error_msg, details))))
                            .await;
                    }
                }
            });
        }
        Action::LoadMoreSearchResults { sid, offset, count } => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.get_search_results(&sid, count, offset).await {
                    Ok(results) => {
                        let _ = tx
                            .send(Action::MoreSearchResultsLoaded(Ok((
                                results.results,
                                offset,
                                results.total,
                            ))))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::MoreSearchResultsLoaded(Err(e.to_string())))
                            .await;
                    }
                }
            });
        }
        Action::CancelJob(sid) => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.cancel_job(&sid).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::JobOperationComplete(format!(
                                "Cancelled job: {}",
                                sid
                            )))
                            .await;
                        // Reload the job list
                        let _ = tx.send(Action::LoadJobs).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Failed to cancel job: {}", e),
                            ))
                            .await;
                        let _ = tx.send(Action::Loading(false)).await;
                    }
                }
            });
        }
        Action::DeleteJob(sid) => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.delete_job(&sid).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::JobOperationComplete(format!(
                                "Deleted job: {}",
                                sid
                            )))
                            .await;
                        // Reload the job list
                        let _ = tx.send(Action::LoadJobs).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Failed to delete job: {}", e),
                            ))
                            .await;
                        let _ = tx.send(Action::Loading(false)).await;
                    }
                }
            });
        }
        Action::CancelJobsBatch(sids) => {
            let _ = tx.send(Action::Loading(true)).await;
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
                        let _ = tx_clone.send(Action::Notify(ToastLevel::Error, err)).await;
                    }
                }

                let _ = tx_clone.send(Action::JobOperationComplete(msg)).await;
                let _ = tx_clone.send(Action::LoadJobs).await;
            });
        }
        Action::DeleteJobsBatch(sids) => {
            let _ = tx.send(Action::Loading(true)).await;
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
                        let _ = tx_clone.send(Action::Notify(ToastLevel::Error, err)).await;
                    }
                }

                let _ = tx_clone.send(Action::JobOperationComplete(msg)).await;
                let _ = tx_clone.send(Action::LoadJobs).await;
            });
        }
        Action::EnableApp(name) => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.enable_app(&name).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Success,
                                format!("App '{}' enabled successfully", name),
                            ))
                            .await;
                        // Refresh apps list
                        let _ = tx.send(Action::LoadApps).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Failed to enable app '{}': {}", name, e),
                            ))
                            .await;
                        let _ = tx.send(Action::Loading(false)).await;
                    }
                }
            });
        }
        Action::DisableApp(name) => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.disable_app(&name).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Success,
                                format!("App '{}' disabled successfully", name),
                            ))
                            .await;
                        // Refresh apps list
                        let _ = tx.send(Action::LoadApps).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Failed to disable app '{}': {}", name, e),
                            ))
                            .await;
                        let _ = tx.send(Action::Loading(false)).await;
                    }
                }
            });
        }
        Action::LoadHealth => {
            let _ = tx.send(Action::Loading(true)).await;
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
                    let _ = tx
                        .send(Action::HealthLoaded(Box::new(Err(combined_error))))
                        .await;
                } else {
                    let _ = tx
                        .send(Action::HealthLoaded(Box::new(Ok(health_output))))
                        .await;
                }
            });
        }
        Action::ExportData(data, path, format) => {
            tokio::spawn(async move {
                let result = splunk_tui::export::export_value(&data, &path, format);

                match result {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Info,
                                format!("Exported to {}", path.display()),
                            ))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Export failed: {}", e),
                            ))
                            .await;
                    }
                }
            });
        }
        // Profile switching actions
        Action::OpenProfileSwitcher => {
            let config_manager_clone = config_manager.clone();
            let tx_popup = tx.clone();
            tokio::spawn(async move {
                let cm = config_manager_clone.lock().await;
                let profiles: Vec<String> = cm.list_profiles().keys().cloned().collect();
                drop(cm); // Release lock before sending actions

                if profiles.is_empty() {
                    let _ = tx_popup
                        .send(Action::Notify(
                            ToastLevel::Error,
                            "No profiles configured. Add profiles using splunk-cli.".to_string(),
                        ))
                        .await;
                } else {
                    // Send the profile list to be opened as a popup
                    let _ = tx_popup
                        .send(Action::OpenProfileSelectorWithList(profiles))
                        .await;
                }
            });
        }
        Action::ProfileSelected(profile_name) => {
            let _ = tx.send(Action::Loading(true)).await;
            let config_manager_clone = config_manager.clone();
            let client_clone = client.clone();
            tokio::spawn(async move {
                let cm = config_manager_clone.lock().await;

                // Get the profile config
                let profiles = cm.list_profiles();
                let Some(profile_config) = profiles.get(&profile_name) else {
                    let _ = tx
                        .send(Action::ProfileSwitchResult(Err(format!(
                            "Profile '{}' not found",
                            profile_name
                        ))))
                        .await;
                    return;
                };

                // Build new config from profile
                let Some(base_url) = profile_config.base_url.clone() else {
                    let _ = tx
                        .send(Action::ProfileSwitchResult(Err(
                            "Profile has no base_url configured".to_string(),
                        )))
                        .await;
                    return;
                };
                let auth_strategy = if let Some(token) = &profile_config.api_token {
                    // API token auth
                    match token.resolve() {
                        Ok(resolved_token) => splunk_client::AuthStrategy::ApiToken {
                            token: resolved_token,
                        },
                        Err(e) => {
                            let _ = tx
                                .send(Action::ProfileSwitchResult(Err(format!(
                                    "Failed to resolve API token: {}",
                                    e
                                ))))
                                .await;
                            return;
                        }
                    }
                } else if let (Some(username), Some(password)) =
                    (&profile_config.username, &profile_config.password)
                {
                    // Session token auth
                    match password.resolve() {
                        Ok(resolved_password) => splunk_client::AuthStrategy::SessionToken {
                            username: username.clone(),
                            password: resolved_password,
                        },
                        Err(e) => {
                            let _ = tx
                                .send(Action::ProfileSwitchResult(Err(format!(
                                    "Failed to resolve password: {}",
                                    e
                                ))))
                                .await;
                            return;
                        }
                    }
                } else {
                    let _ = tx
                        .send(Action::ProfileSwitchResult(Err(
                            "Profile has no authentication configured".to_string(),
                        )))
                        .await;
                    return;
                };

                // Build new client
                let mut new_client = match SplunkClient::builder()
                    .base_url(base_url.clone())
                    .auth_strategy(auth_strategy)
                    .skip_verify(profile_config.skip_verify.unwrap_or(false))
                    .timeout(std::time::Duration::from_secs(
                        profile_config.timeout_seconds.unwrap_or(30),
                    ))
                    .build()
                {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx
                            .send(Action::ProfileSwitchResult(Err(format!(
                                "Failed to build client: {}",
                                e
                            ))))
                            .await;
                        return;
                    }
                };

                // Authenticate if using session tokens
                if !new_client.is_api_token_auth()
                    && let Err(e) = new_client.login().await
                {
                    let _ = tx
                        .send(Action::ProfileSwitchResult(Err(format!(
                            "Authentication failed: {}",
                            e
                        ))))
                        .await;
                    return;
                }

                // Replace the shared client
                let mut client_guard = client_clone.lock().await;
                *client_guard = new_client;
                drop(client_guard);

                // Determine auth mode display string
                let auth_mode = if profile_config.api_token.is_some() {
                    "token".to_string()
                } else if let Some(username) = &profile_config.username {
                    format!("session ({})", username)
                } else {
                    "unknown".to_string()
                };

                // Send success result
                let ctx = ConnectionContext {
                    profile_name: Some(profile_name.clone()),
                    base_url,
                    auth_mode,
                };
                let _ = tx.send(Action::ProfileSwitchResult(Ok(ctx))).await;

                // Clear all cached data
                let _ = tx.send(Action::ClearAllData).await;

                // Trigger reload for current screen
                let _ = tx.send(Action::Loading(false)).await;
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
