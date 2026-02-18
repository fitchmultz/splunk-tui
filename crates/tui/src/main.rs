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
//! - Bootstrap mode allows UI to start without valid auth credentials (RQ-0454).

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
use splunk_tui::runtime::config::ConfigLoadResult;
use splunk_tui::runtime::startup::{
    BootstrapReason, StartupPhase, action_requires_client, should_launch_tutorial,
};
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
    SearchDefaultConfig, SearchDefaults,
};

use splunk_tui::runtime::{
    client::create_client,
    config::{load_config_with_defaults, save_and_quit, try_load_config_with_bootstrap_fallback},
    side_effects::{TaskTracker, handle_side_effects},
    terminal::TerminalGuard,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Capture no_mouse flag for later use in cleanup
    let no_mouse = cli.no_mouse;

    // Create logs directory if it doesn't exist
    std::fs::create_dir_all(&cli.log_dir)?;

    // Initialize OpenTelemetry if OTLP endpoint is configured
    let _otel_guard = if let Some(ref endpoint) = cli.otlp_endpoint {
        let service_name = cli
            .otel_service_name
            .clone()
            .unwrap_or_else(|| "splunk-tui".to_string());

        let config = splunk_client::TracingConfig::new()
            .with_otlp_endpoint(endpoint)
            .with_service_name(service_name)
            .with_stdout(false); // TUI uses file logging, not stdout

        match config.init() {
            Ok(guard) => {
                tracing::info!("OpenTelemetry tracing enabled: endpoint={}", endpoint);
                Some(guard)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize OpenTelemetry: {}", e);
                None
            }
        }
    } else {
        // Initialize file-based logging with configurable directory
        let log_file_name = "splunk-tui.log";
        let file_appender = tracing_appender::rolling::daily(&cli.log_dir, log_file_name);
        let (non_blocking, _guard) = non_blocking(file_appender);

        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(fmt::layer().with_writer(non_blocking))
            .init();
        None
    };

    // Note: _guard must live for entire main() duration to ensure logs are flushed

    // Initialize metrics exporter if --metrics-bind is provided
    let _metrics_exporter = if let Some(ref bind_addr) = cli.metrics_bind {
        match splunk_client::MetricsExporter::install(bind_addr) {
            Ok(exporter) => {
                tracing::info!("Metrics exporter started on http://{}/metrics", bind_addr);
                Some(exporter)
            }
            Err(e) => {
                tracing::error!("Failed to start metrics exporter: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Store whether metrics are enabled for use in spawned tasks
    let metrics_enabled = _metrics_exporter.is_some();

    // Try to load configuration with bootstrap fallback (RQ-0454)
    // This allows the UI to start even if auth is missing/invalid
    struct StartupState {
        phase: StartupPhase,
        client: Option<Arc<splunk_client::SplunkClient>>,
        search_defaults: SearchDefaultConfig,
        internal_logs_defaults: InternalLogsDefaults,
        bootstrap_reason: Option<BootstrapReason>,
        connection_ctx: ConnectionContext,
    }

    let startup_state = match try_load_config_with_bootstrap_fallback(&cli)? {
        ConfigLoadResult::Success {
            search_defaults,
            internal_logs_defaults,
            config,
            resolved_profile_name,
        } => {
            // Config loaded successfully - try to create client
            match create_client(&config).await {
                Ok(new_client) => {
                    tracing::info!("Successfully authenticated with Splunk server");
                    let auth_mode = match &config.auth.strategy {
                        ConfigAuthStrategy::ApiToken { .. } => "token".to_string(),
                        ConfigAuthStrategy::SessionToken { username, .. } => {
                            format!("session ({username})")
                        }
                    };
                    let connection_ctx = ConnectionContext {
                        profile_name: resolved_profile_name.clone(),
                        base_url: config.connection.base_url.clone(),
                        auth_mode,
                    };
                    StartupState {
                        phase: StartupPhase::Main,
                        client: Some(Arc::new(new_client)),
                        search_defaults,
                        internal_logs_defaults,
                        bootstrap_reason: None,
                        connection_ctx,
                    }
                }
                Err(e) => {
                    // Authentication failed - enter bootstrap mode
                    tracing::warn!("Authentication failed, entering bootstrap mode: {}", e);
                    StartupState {
                        phase: StartupPhase::Bootstrap {
                            reason: BootstrapReason::InvalidAuth,
                        },
                        client: None,
                        search_defaults,
                        internal_logs_defaults,
                        bootstrap_reason: Some(BootstrapReason::InvalidAuth),
                        connection_ctx: ConnectionContext {
                            profile_name: None,
                            base_url: "Not connected".to_string(),
                            auth_mode: "bootstrap".to_string(),
                        },
                    }
                }
            }
        }
        ConfigLoadResult::Bootstrap {
            reason,
            search_defaults,
            internal_logs_defaults,
        } => {
            // Missing/invalid auth - enter bootstrap mode
            tracing::info!(
                "Entering bootstrap mode due to missing/invalid auth: {:?}",
                reason
            );
            StartupState {
                phase: StartupPhase::Bootstrap { reason },
                client: None,
                search_defaults,
                internal_logs_defaults,
                bootstrap_reason: Some(reason),
                connection_ctx: ConnectionContext {
                    profile_name: None,
                    base_url: "Not connected".to_string(),
                    auth_mode: "bootstrap".to_string(),
                },
            }
        }
    };

    // Destructure the startup state
    let StartupState {
        phase,
        client,
        search_defaults,
        internal_logs_defaults,
        bootstrap_reason,
        connection_ctx,
    } = startup_state;

    let mut startup_phase = phase;
    let mut client: Option<std::sync::Arc<splunk_client::SplunkClient>> = client;

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
    use secrecy::SecretString;
    use splunk_config::encryption::MasterKeySource;
    let source = if let Some(ref pw) = cli.config_password {
        MasterKeySource::Password(SecretString::new(pw.clone().into()))
    } else if let Some(ref var) = cli.config_key_var {
        MasterKeySource::Env(var.clone())
    } else {
        MasterKeySource::Keyring
    };

    let config_manager = if let Some(config_path) = &cli.config_path {
        let path_str = config_path.to_string_lossy();
        if !path_str.trim().is_empty() {
            ConfigManager::new_with_path_and_source(config_path.clone(), source)?
        } else {
            ConfigManager::new_with_path_and_source(
                splunk_config::persistence::default_config_path()?,
                source,
            )?
        }
    } else {
        ConfigManager::new_with_path_and_source(
            splunk_config::persistence::default_config_path()?,
            source,
        )?
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
    let is_first_run = should_launch_tutorial(
        config_manager_for_first_run.list_profiles().is_empty(),
        cli.skip_tutorial,
        persisted_state.tutorial_completed,
    );
    drop(config_manager_for_first_run); // Release lock before creating app

    // Apply environment variable overrides to search defaults
    // Precedence: env vars > persisted values > hardcoded defaults
    // Sanitize to ensure invariants (non-empty times, max_results >= 1) are enforced
    persisted_state.search_defaults = SearchDefaults {
        earliest_time: search_defaults.earliest_time,
        latest_time: search_defaults.latest_time,
        max_results: search_defaults.max_results,
    }
    .sanitize();

    // Apply environment variable overrides to internal logs defaults
    // Precedence: env vars > persisted values > hardcoded defaults
    // Sanitize to ensure invariants (count > 0, non-empty earliest_time) are enforced
    persisted_state.internal_logs_defaults = InternalLogsDefaults {
        count: internal_logs_defaults.count,
        earliest_time: internal_logs_defaults.earliest_time,
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

    // Create app with persisted state and pre-built connection context
    let mut app = App::new(Some(persisted_state), connection_ctx);

    // Enable UX telemetry collection when metrics exporter is enabled
    app.ux_telemetry = Some(splunk_tui::ux_telemetry::UxTelemetryCollector::new(
        metrics_enabled,
    ));

    // Track session start for onboarding checklist (increments session count,
    // resets hint counters, and updates sessions_since_completion)
    app.on_session_start();

    // Set bootstrap message if in bootstrap mode
    if let Some(reason) = bootstrap_reason {
        app.toasts
            .push(splunk_tui::ui::Toast::warning(reason.to_string()));
    }

    // Launch tutorial on first run
    // This now works in bootstrap mode - tutorial opens before auth is required
    if is_first_run {
        app.popup = Some(
            Popup::builder(PopupType::TutorialWizard {
                state: TutorialState::new(),
            })
            .build(),
        );
    }

    // Track if health check task is already running to prevent duplicates
    let mut health_check_running = client.is_some();

    // Spawn background health monitoring task (only if we have a client)
    if let Some(client_health) = client.clone() {
        let tx_health = tx.clone();
        // Get health check interval from client or use default
        let health_check_interval = 60; // Default 60s
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
    }

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
        // Record frame render duration
        let render_start = std::time::Instant::now();
        terminal.draw(|f| app.render(f))?;
        let render_duration = render_start.elapsed();

        // Record TUI frame render duration metric if metrics are enabled
        if metrics_enabled {
            metrics::histogram!("splunk_tui_frame_render_duration_seconds")
                .record(render_duration.as_secs_f64());
        }

        tokio::select! {
            Some(action) = rx.recv() => {
                // Record action queue depth metric if metrics are enabled
                if metrics_enabled {
                    let queue_depth = rx.len();
                    metrics::gauge!("splunk_tui_action_queue_depth").set(queue_depth as f64);
                }

                tracing::info!("Handling action: {:?}", splunk_tui::action::RedactedAction(&action));

                // Check for quit first
                if matches!(action, Action::Quit) {
                    if let Err(e) = save_and_quit(&app, &config_manager).await {
                        tracing::error!(error = %e, "Failed to save config");
                    }
                    break;
                }

                // Handle bootstrap connect request
                if matches!(action, Action::BootstrapConnectRequested) {
                    // Prevent multiple concurrent connection attempts
                    match startup_phase {
                        StartupPhase::Bootstrap { .. } => {
                            startup_phase = StartupPhase::Connecting;
                            app.loading = true;

                            // Attempt to load config and create client
                            let config_result = load_config_with_defaults(&cli);
                            let tx_connect = tx.clone();

                            tokio::spawn(async move {
                                match config_result {
                                    Ok((_, _, config, resolved_profile_name)) => {
                                        match create_client(&config).await {
                                            Ok(new_client) => {
                                                // Emit bootstrap connect success metric
                                                if metrics_enabled {
                                                    metrics::counter!(
                                                        splunk_client::metrics::METRIC_UX_BOOTSTRAP_CONNECT,
                                                        "success" => "true",
                                                        "reason" => "connected",
                                                    ).increment(1);
                                                }

                                                // Build connection context
                                                let auth_mode = match &config.auth.strategy {
                                                    ConfigAuthStrategy::ApiToken { .. } => "token".to_string(),
                                                    ConfigAuthStrategy::SessionToken { username, .. } => {
                                                        format!("session ({username})")
                                                    }
                                                };
                                                let connection_ctx = ConnectionContext {
                                                    profile_name: resolved_profile_name,
                                                    base_url: config.connection.base_url.clone(),
                                                    auth_mode,
                                                };

                                                let _ = tx_connect.send(Action::EnterMainMode {
                                                    client: Arc::new(new_client),
                                                    connection_ctx,
                                                }).await;
                                            }
                                            Err(e) => {
                                                // Emit bootstrap connect failure metric
                                                if metrics_enabled {
                                                    let reason = if e.to_string().to_lowercase().contains("auth") {
                                                        "invalid_auth"
                                                    } else {
                                                        "connection_error"
                                                    };
                                                    metrics::counter!(
                                                        splunk_client::metrics::METRIC_UX_BOOTSTRAP_CONNECT,
                                                        "success" => "false",
                                                        "reason" => reason,
                                                    ).increment(1);
                                                }

                                                let _ = tx_connect.send(Action::BootstrapConnectFinished {
                                                    ok: false,
                                                    error: Some(e.to_string()),
                                                }).await;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        // Emit bootstrap connect failure metric (config error)
                                        if metrics_enabled {
                                            let reason = if e.to_string().to_lowercase().contains("auth") {
                                                "missing_auth"
                                            } else {
                                                "config_error"
                                            };
                                            metrics::counter!(
                                                splunk_client::metrics::METRIC_UX_BOOTSTRAP_CONNECT,
                                                "success" => "false",
                                                "reason" => reason,
                                            ).increment(1);
                                        }

                                        let _ = tx_connect.send(Action::BootstrapConnectFinished {
                                            ok: false,
                                            error: Some(e.to_string()),
                                        }).await;
                                    }
                                }
                            });
                        }
                        StartupPhase::Connecting => {
                            // Already connecting - show info toast and skip
                            app.toasts.push(splunk_tui::ui::Toast::info(
                                "Connection already in progress...".to_string()
                            ));
                        }
                        _ => {
                            // Not in bootstrap/connecting - ignore
                            tracing::debug!("Ignoring BootstrapConnectRequested in {:?} phase", startup_phase);
                        }
                    }
                    continue;
                }

                // Handle bootstrap connect finished (error case only)
                let is_bootstrap_finished = matches!(action, Action::BootstrapConnectFinished { .. });
                if is_bootstrap_finished {
                    if let Action::BootstrapConnectFinished { ok, error } = &action {
                        app.loading = false;
                        if !*ok {
                            // Connection failed - stay in bootstrap mode
                            startup_phase = StartupPhase::Bootstrap {
                                reason: BootstrapReason::InvalidAuth,
                            };
                            if let Some(err) = error {
                                app.toasts.push(splunk_tui::ui::Toast::error(format!(
                                    "Connection failed: {}",
                                    err
                                )));
                            }
                        }
                    }
                    continue;
                }

                // Handle enter main mode (bootstrap -> main transition)
                let is_enter_main = matches!(action, Action::EnterMainMode { .. });
                if is_enter_main {
                    if let Action::EnterMainMode { client: new_client, connection_ctx } = action {
                        app.loading = false;
                        startup_phase = StartupPhase::Main;
                        client = Some(new_client);
                        // Update app connection context
                        app.profile_name = connection_ctx.profile_name.clone();
                        app.base_url = Some(connection_ctx.base_url.clone());
                        app.auth_mode = Some(connection_ctx.auth_mode.clone());
                        app.toasts.push(splunk_tui::ui::Toast::success(
                            "Connected successfully! Welcome to Splunk TUI.".to_string()
                        ));

                        // Spawn health check task now that we have a client (if not already running)
                        if !health_check_running {
                            if let Some(ref client_health) = client {
                                health_check_running = true;
                                let tx_health = tx.clone();
                                let health_check_interval = 60; // Default 60s
                                let client_health = client_health.clone();
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
                                                        tracing::debug!("Health status channel full, dropping update");
                                                    }
                                                    Err(TrySendError::Closed(_)) => {
                                                        break;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                match tx_health.try_send(Action::HealthStatusLoaded(Err(Arc::new(e)))) {
                                                    Ok(()) => {}
                                                    Err(TrySendError::Full(_)) => {
                                                        tracing::debug!("Health status channel full, dropping update");
                                                    }
                                                    Err(TrySendError::Closed(_)) => {
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    continue;
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

                        // Only handle side effects if we have a client or action doesn't require one
                        if client.is_some() || !action_requires_client(&a) {
                            if let Some(ref c) = client {
                                handle_side_effects(a, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                            }
                        }

                        // Execute follow-up action for workload pagination if derived
                        if let Some(followup) = followup_action {
                            if let Some(ref c) = client {
                                handle_side_effects(followup, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                            }
                        }

                        // If navigation action, trigger load for new screen
                        if is_navigation
                            && let Some(load_action) = app.load_action_for_screen()
                        {
                            if let Some(ref c) = client {
                                handle_side_effects(load_action, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                            }
                        }
                        // Check if we need to load more results after navigation
                        if let Some(load_action) = app.maybe_fetch_more_results() {
                            if let Some(ref c) = client {
                                handle_side_effects(load_action, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                            }
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

                        // Only handle side effects if we have a client or action doesn't require one
                        if let Some(ref c) = client {
                            handle_side_effects(a, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                        }

                        // If navigation action, trigger load for new screen
                        if is_navigation
                            && let Some(load_action) = app.load_action_for_screen()
                        {
                            if let Some(ref c) = client {
                                handle_side_effects(load_action, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                            }
                        }
                        // Check if we need to load more results after navigation
                        if let Some(load_action) = app.maybe_fetch_more_results() {
                            if let Some(ref c) = client {
                                handle_side_effects(load_action, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                            }
                        }
                    }
                } else {
                    let was_toggle = matches!(action, Action::ToggleClusterViewMode);
                    let was_profile_switch = matches!(action, Action::ProfileSwitchResult(Ok(_)));
                    app.update(action.clone());

                    // Only handle side effects if we have a client or action doesn't require one
                    if let Some(ref c) = client {
                        handle_side_effects(action, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                    }

                    // After toggle, if we're now in Peers view, trigger peers load
                    if was_toggle && app.cluster_view_mode == splunk_tui::app::ClusterViewMode::Peers {
                        if let Some(ref c) = client {
                            handle_side_effects(Action::LoadClusterPeers, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                        }
                    }
                    // After successful profile switch, trigger reload for current screen
                    if was_profile_switch
                        && let Some(load_action) = app.load_action_for_screen()
                    {
                        if let Some(ref c) = client {
                            handle_side_effects(load_action, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                        }
                    }
                }
            }
            _ = tick_interval.tick() => {
                // Always process tick for TTL pruning and animations
                app.update(Action::Tick);
            }
            _ = refresh_interval.tick() => {
                // Data refresh is separate from UI tick
                // Only process if we have a client (not in bootstrap mode)
                if client.is_some() {
                    if let Some(a) = app.handle_tick() {
                        app.update(a.clone());
                        if let Some(ref c) = client {
                            handle_side_effects(a, c.clone(), tx.clone(), config_manager.clone(), task_tracker.clone()).await;
                        }
                    }
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

    // Shutdown OpenTelemetry to flush pending spans
    if let Some(guard) = _otel_guard {
        guard.shutdown();
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
