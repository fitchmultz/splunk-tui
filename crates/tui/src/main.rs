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
//! Invariants:
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
use splunk_tui::onboarding::TutorialState;
use splunk_tui::ui::popup::{Popup, PopupType};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::channel};
use tracing_appender::non_blocking;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use splunk_config::constants::{
    DEFAULT_CHANNEL_CAPACITY, DEFAULT_REFRESH_INTERVAL_SECS, DEFAULT_UI_TICK_MS,
};
use splunk_config::{
    AuthStrategy as ConfigAuthStrategy, ConfigManager, InternalLogsDefaults, PersistedState,
    SearchDefaults,
};

use splunk_tui::runtime::{
    client::create_client,
    config::{load_config_with_defaults, save_and_quit},
    side_effects::{SharedClient, TaskTracker, handle_side_effects},
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
    // Also get search defaults and internal logs defaults with env var overrides applied
    let (search_default_config, internal_logs_default_config, config, resolved_profile_name) =
        load_config_with_defaults(&cli)?;

    // Warn if using default credentials (security check)
    if config.is_using_default_credentials() {
        tracing::warn!(
            "Using default Splunk credentials (admin/changeme). \
             These are for local development only - change before production use."
        );
    }

    // Build and authenticate client
    let client: SharedClient = Arc::new(create_client(&config).await?);

    // Create task tracker for managing spawned tasks
    let task_tracker = TaskTracker::new();

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
    task_tracker.spawn(async move {
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
                        crossterm::event::Event::Resize(width, height) => {
                            Some(Action::Resize(width, height))
                        }
                        _ => None,
                    };

                    if let Some(action) = action {
                        // Different backpressure strategy based on event type:
                        // - Key/Resize: Use blocking send to ensure user intent is never lost
                        // - Mouse: Use try_send and drop if full (prevents mouse move flooding)
                        let is_critical = matches!(
                            event,
                            crossterm::event::Event::Key(_) | crossterm::event::Event::Resize(_, _)
                        );

                        if is_critical {
                            // Critical user intent events - await until space available
                            if tx_input.send(action).await.is_err() {
                                // Channel closed, exit task
                                break;
                            }
                        } else {
                            // Mouse events are droppable (especially mouse move floods)
                            match tx_input.try_send(action) {
                                Ok(()) => {}
                                Err(TrySendError::Full(_)) => {
                                    tracing::debug!("Input channel full, dropping mouse event");
                                }
                                Err(TrySendError::Closed(_)) => {
                                    // Channel closed, exit task
                                    break;
                                }
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
    // CLI --config-path takes precedence (if not blank); ConfigManager uses the same resolution logic
    let config_manager = if let Some(config_path) = &cli.config_path {
        let path_str = config_path.to_string_lossy();
        if !path_str.trim().is_empty() {
            ConfigManager::new_with_path(config_path.clone())?
        } else {
            ConfigManager::new()?
        }
    } else {
        ConfigManager::new()?
    };
    let mut persisted_state = if cli.fresh {
        tracing::info!("--fresh flag set, starting with default state");
        PersistedState::default()
    } else {
        config_manager.load()
    };
    let config_manager = Arc::new(Mutex::new(config_manager));

    // Check if this is first run (no profiles exist and tutorial not completed)
    let config_manager_for_first_run = config_manager.lock().await;
    let is_first_run = config_manager_for_first_run.list_profiles().is_empty()
        && !cli.skip_tutorial
        && !persisted_state.tutorial_completed;
    drop(config_manager_for_first_run); // Release lock before creating app

    // Apply environment variable overrides to search defaults
    // Precedence: env vars > persisted values > hardcoded defaults
    // Sanitize to ensure invariants (non-empty times, max_results >= 1) are enforced
    persisted_state.search_defaults = SearchDefaults {
        earliest_time: search_default_config.earliest_time,
        latest_time: search_default_config.latest_time,
        max_results: search_default_config.max_results,
    }
    .sanitize();

    // Apply environment variable overrides to internal logs defaults
    // Precedence: env vars > persisted values > hardcoded defaults
    // Sanitize to ensure invariants (count > 0, non-empty earliest_time) are enforced
    persisted_state.internal_logs_defaults = InternalLogsDefaults {
        count: internal_logs_default_config.count,
        earliest_time: internal_logs_default_config.earliest_time,
    }
    .sanitize();

    // Initialize keybinding overrides from persisted state
    if let Err(e) =
        splunk_tui::input::keymap::overrides::init_overrides(&persisted_state.keybind_overrides)
    {
        tracing::warn!(
            "Failed to initialize keybinding overrides: {}. Using defaults.",
            e
        );
    }

    // Initialize footer hints cache to avoid per-frame allocations (RQ-0336)
    splunk_tui::input::keymap::init_footer_cache();

    // Build connection context for TUI header display (RQ-0134)
    let connection_ctx = ConnectionContext {
        profile_name: resolved_profile_name,
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

    // Launch tutorial on first run
    if is_first_run {
        app.popup = Some(
            Popup::builder(PopupType::TutorialWizard {
                state: TutorialState::new(),
            })
            .build(),
        );
    }

    // Spawn background health monitoring task (configurable interval, default 60s)
    let tx_health = tx.clone();
    let client_health = client.clone();
    let health_check_interval = config.connection.health_check_interval_seconds;
    task_tracker.spawn(async move {
        use tokio::sync::mpsc::error::TrySendError;

        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(health_check_interval));
        loop {
            interval.tick().await;
            match client_health.get_health().await {
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

    // Create auto-save interval (every 30 seconds)
    const AUTO_SAVE_INTERVAL_SECS: u64 = 30;
    let mut auto_save_interval =
        tokio::time::interval(tokio::time::Duration::from_secs(AUTO_SAVE_INTERVAL_SECS));

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

                // Handle PersistState specially - needs access to app
                if matches!(action, Action::PersistState) {
                    let state = app.get_persisted_state();
                    let cm = config_manager.clone();
                    tokio::task::spawn(async move {
                        let mut manager = cm.lock().await;
                        if let Err(e) = manager.save(&state) {
                            tracing::error!("Failed to persist state: {}", e);
                        }
                    });
                    continue;
                }

                // Handle LoadMore* actions by converting to Load* with pagination params
                let action = app.translate_load_more_action(action);
                // Handle Refresh* actions by converting to Load* with offset=0
                let action = app.translate_refresh_action(action);

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

                        // Translate LoadMore* actions produced by input handlers
                        let a = app.translate_load_more_action(a);

                        // Handle any additional follow-up actions for pagination
                        let followup_action = match a {
                            Action::LoadWorkloadPools { .. } => {
                                // If this was translated from LoadMoreWorkloadPools,
                                // we don't need an additional follow-up
                                None
                            }
                            Action::LoadWorkloadRules { .. } => {
                                // If this was translated from LoadMoreWorkloadRules,
                                // we don't need an additional follow-up
                                None
                            }
                            _ => None,
                        };

                        let is_navigation = matches!(a, Action::NextScreen | Action::PreviousScreen);
                        app.update(a.clone());
                        handle_side_effects(a, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;

                        // Execute follow-up action for workload pagination if derived
                        if let Some(followup) = followup_action {
                            handle_side_effects(followup, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                        }

                        // If navigation action, trigger load for new screen
                        if is_navigation
                            && let Some(load_action) = app.load_action_for_screen()
                        {
                            handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                        }
                        // Check if we need to load more results after navigation
                        if let Some(load_action) = app.maybe_fetch_more_results() {
                            handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
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
                        // Translate LoadMore* actions produced by mouse handlers
                        let a = app.translate_load_more_action(a);
                        let is_navigation = matches!(a, Action::NextScreen | Action::PreviousScreen);
                        app.update(a.clone());
                        handle_side_effects(a, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                        // If navigation action, trigger load for new screen
                        if is_navigation
                            && let Some(load_action) = app.load_action_for_screen()
                        {
                            handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                        }
                        // Check if we need to load more results after navigation
                        if let Some(load_action) = app.maybe_fetch_more_results() {
                            handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                        }
                    }
                } else {
                    let was_toggle = matches!(action, Action::ToggleClusterViewMode);
                    let was_profile_switch = matches!(action, Action::ProfileSwitchResult(Ok(_)));
                    app.update(action.clone());
                    handle_side_effects(action, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                    // After toggle, if we're now in Peers view, trigger peers load
                    if was_toggle && app.cluster_view_mode == splunk_tui::app::ClusterViewMode::Peers {
                        handle_side_effects(Action::LoadClusterPeers, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                    }
                    // After successful profile switch, trigger reload for current screen
                    if was_profile_switch
                        && let Some(load_action) = app.load_action_for_screen()
                    {
                        handle_side_effects(load_action, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
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
                    handle_side_effects(a, client.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                }
            }
            _ = auto_save_interval.tick() => {
                // Periodic auto-save of persisted state
                let state = app.get_persisted_state();
                let cm = config_manager.clone();
                tokio::task::spawn(async move {
                    let mut manager = cm.lock().await;
                    if let Err(e) = manager.save(&state) {
                        tracing::error!("Failed to auto-save state: {}", e);
                    } else {
                        tracing::debug!("State auto-saved successfully");
                    }
                });
            }
        }
    }

    // Graceful shutdown: close tracker and wait for tasks
    let _ = task_tracker.close();
    task_tracker.wait().await;

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
