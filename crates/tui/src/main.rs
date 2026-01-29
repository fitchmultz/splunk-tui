//! Splunk TUI - Terminal user interface for Splunk Enterprise.
//!
//! Responsibilities:
//! - Orchestrate application startup and shutdown.
//! - Initialize terminal, logging, and async runtime.
//! - Run the main event loop.
//!
//! Does NOT handle:
//! - Core business logic or REST API implementation (see `crates/client`).
//! - Manual configuration file editing (see `crates/cli`).
//! - Configuration persistence (see `crates/config`).
//! - Async API calls (see `runtime::side_effects`).
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
use splunk_tui::action::Action;
use splunk_tui::app::{App, ConnectionContext};
use splunk_tui::cli::Cli;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::channel};
use tracing_appender::non_blocking;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use splunk_config::constants::{
    DEFAULT_CHANNEL_CAPACITY, DEFAULT_REFRESH_INTERVAL_SECS, DEFAULT_UI_TICK_MS,
};
use splunk_config::{
    AuthStrategy as ConfigAuthStrategy, ConfigManager, SearchDefaults, env_var_or_none,
};

use splunk_tui::runtime::{
    client::create_client,
    config::{load_config_with_search_defaults, save_and_quit},
    side_effects::{SharedClient, handle_side_effects},
    terminal::TerminalGuard,
};

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

    // Warn if using default credentials (security check)
    if config.is_using_default_credentials() {
        tracing::warn!(
            "Using default Splunk credentials (admin/changeme). \
             These are for local development only - change before production use."
        );
    }

    // Build and authenticate client
    let client: SharedClient = Arc::new(Mutex::new(create_client(&config).await?));

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
    const ACTION_CHANNEL_CAPACITY: usize = DEFAULT_CHANNEL_CAPACITY;
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
                    match tx_health.try_send(Action::HealthStatusLoaded(Err(Arc::new(e)))) {
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

    // Create UI tick interval for smooth animations
    let mut tick_interval =
        tokio::time::interval(tokio::time::Duration::from_millis(DEFAULT_UI_TICK_MS));

    // Create data refresh interval (decoupled from UI tick)
    let mut refresh_interval = tokio::time::interval(tokio::time::Duration::from_secs(
        DEFAULT_REFRESH_INTERVAL_SECS,
    ));

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
