//! Common test utilities for TUI side effects tests.
//!
//! This module provides shared helper functions and types for testing the TUI's
//! async side effect handlers. It uses wiremock to mock HTTP responses from
//! the Splunk REST API.
//!
//! # Invariants
//! - Fixtures are loaded from the client's fixtures directory
//! - All mock servers use random available ports to avoid conflicts
//! - Each test gets its own isolated mock server and action channel
//!
//! # What this does NOT handle
//! - Actual HTTP requests to real Splunk servers (use live tests for that)
//! - TUI rendering or terminal management
//! - Configuration file I/O (use temp directories for that)

// Allow dead code since not all tests use all utilities
#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

/// Load a JSON fixture file from the client's fixtures directory.
///
/// # Arguments
/// * `fixture_path` - Relative path within the fixtures directory (e.g., "indexes/list_indexes.json")
///
/// # Panics
/// - If the fixture file cannot be read
/// - If the file content is not valid JSON
pub fn load_fixture(fixture_path: &str) -> serde_json::Value {
    // Fixtures are in the client crate's fixtures directory
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir
        .parent()
        .expect("No parent directory")
        .join("client")
        .join("fixtures");
    let full_path = fixture_dir.join(fixture_path);
    let content = std::fs::read_to_string(&full_path)
        .unwrap_or_else(|_| panic!("Failed to load fixture: {}", full_path.display()));
    serde_json::from_str(&content).expect("Invalid JSON in fixture")
}

// Re-export commonly used types for test convenience
pub use splunk_client::{AuthStrategy, SplunkClient};
pub use splunk_config::ConfigManager;
pub use splunk_tui::action::Action;
pub use splunk_tui::runtime::side_effects::{SharedClient, handle_side_effects};
pub use tokio::sync::mpsc::{Receiver, Sender};
pub use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test harness for side effects testing.
///
/// Provides a mock HTTP server, action channel, and shared client
/// for testing async side effect handlers in isolation.
pub struct SideEffectsTestHarness {
    /// The mock HTTP server for intercepting API calls
    pub mock_server: MockServer,
    /// Receiver for actions sent by the side effect handlers
    pub action_rx: Receiver<Action>,
    /// Sender for actions (clone this to pass to handlers)
    pub action_tx: Sender<Action>,
    /// Shared Splunk client pointing to the mock server
    pub client: SharedClient,
    /// Configuration manager for profile operations
    pub config_manager: Arc<Mutex<ConfigManager>>,
}

impl SideEffectsTestHarness {
    /// Create a new test harness with a mock server and fresh channels.
    ///
    /// # Returns
    /// A fully configured test harness ready for testing side effects.
    pub async fn new() -> Self {
        let mock_server = MockServer::start().await;
        let (action_tx, action_rx) = mpsc::channel::<Action>(100);

        let client = create_test_client(&mock_server.uri()).await;
        let config_manager = create_test_config_manager().await;

        Self {
            mock_server,
            action_rx,
            action_tx,
            client,
            config_manager,
        }
    }

    /// Handle an action and collect all resulting actions.
    ///
    /// This calls `handle_side_effects` directly under a short timeout (to detect
    /// blocking behavior), then collects all actions sent by spawned tasks.
    ///
    /// # Arguments
    /// * `action` - The action to handle
    /// * `timeout_secs` - Maximum time to wait for actions (in seconds)
    ///
    /// # Returns
    /// A vector of all actions sent by the handler (in order received)
    pub async fn handle_and_collect(&mut self, action: Action, timeout_secs: u64) -> Vec<Action> {
        // Call handle_side_effects directly under a short timeout.
        // This ensures the function returns promptly and does not block on network I/O.
        // Any blocking await will cause this to timeout and fail the test.
        let client = self.client.clone();
        let tx = self.action_tx.clone();
        let config_manager = self.config_manager.clone();

        let handle_future = handle_side_effects(action, client, tx, config_manager);
        match tokio::time::timeout(tokio::time::Duration::from_millis(100), handle_future).await {
            Ok(()) => {}
            Err(_) => {
                panic!(
                    "handle_side_effects timed out - it may be blocking on network I/O instead of spawning tasks"
                );
            }
        }

        // Give spawned tasks a chance to start without real-time delay
        tokio::task::yield_now().await;

        // Collect actions until timeout
        let mut actions = Vec::new();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);

        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(
                tokio::time::Duration::from_millis(100),
                self.action_rx.recv(),
            )
            .await
            {
                Ok(Some(action)) => actions.push(action),
                Ok(None) => break, // Channel closed
                Err(_) => {
                    // Timeout - check if there are any pending tasks
                    tokio::task::yield_now().await;
                }
            }
        }

        actions
    }

    /// Expect a specific action within a timeout.
    ///
    /// # Arguments
    /// * `timeout_ms` - Maximum time to wait in milliseconds
    ///
    /// # Returns
    /// The received action, or panics if timeout exceeded
    pub async fn expect_action(&mut self, timeout_ms: u64) -> Action {
        tokio::time::timeout(
            tokio::time::Duration::from_millis(timeout_ms),
            self.action_rx.recv(),
        )
        .await
        .expect("Timeout waiting for action")
        .expect("Channel closed while waiting for action")
    }

    /// Drain all pending actions from the channel.
    ///
    /// # Returns
    /// A vector of all actions currently in the channel
    pub async fn drain_actions(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
        while let Ok(Some(action)) = tokio::time::timeout(
            tokio::time::Duration::from_millis(10),
            self.action_rx.recv(),
        )
        .await
        {
            actions.push(action);
        }
        actions
    }
}

/// Create a test Splunk client pointing to the mock server.
///
/// # Arguments
/// * `mock_uri` - The base URI of the mock server
///
/// # Returns
/// A shared client wrapped in Arc<Mutex<>>
pub async fn create_test_client(mock_uri: &str) -> SharedClient {
    let client = SplunkClient::builder()
        .base_url(mock_uri.to_string())
        .auth_strategy(AuthStrategy::ApiToken {
            token: secrecy::SecretString::new("test-token".to_string().into()),
        })
        .skip_verify(true)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("Failed to build test client");

    Arc::new(Mutex::new(client))
}

/// Create a test configuration manager.
///
/// # Returns
/// A shared config manager with a temporary directory
pub async fn create_test_config_manager() -> Arc<Mutex<ConfigManager>> {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.toml");

    let config_manager =
        ConfigManager::new_with_path(config_path).expect("Failed to create config manager");

    // Keep temp_dir alive by storing it in the config manager's directory
    // Note: This is a bit of a hack - in real tests you may want to manage
    // the temp directory separately
    Arc::new(Mutex::new(config_manager))
}

/// Mount a mock response for a specific endpoint.
///
/// # Arguments
/// * `server` - The mock server to mount on
/// * `method` - HTTP method (e.g., "GET", "POST")
/// * `path` - URL path to match
/// * `fixture` - Fixture data to return
/// * `status` - HTTP status code (default: 200)
pub async fn mock_endpoint(
    server: &MockServer,
    method: &str,
    path: &str,
    fixture: serde_json::Value,
    status: u16,
) {
    use wiremock::matchers::{method as method_matcher, path as path_matcher};

    Mock::given(method_matcher(method))
        .and(path_matcher(path))
        .respond_with(ResponseTemplate::new(status).set_body_json(fixture))
        .mount(server)
        .await;
}

/// Mount an error response for a specific endpoint.
///
/// # Arguments
/// * `server` - The mock server to mount on
/// * `method` - HTTP method
/// * `path` - URL path to match
/// * `status` - HTTP error status code
/// * `error_body` - Optional error response body
pub async fn mock_endpoint_error(
    server: &MockServer,
    method: &str,
    path: &str,
    status: u16,
    error_body: Option<String>,
) {
    use wiremock::matchers::{method as method_matcher, path as path_matcher};

    let template = if let Some(body) = error_body {
        ResponseTemplate::new(status).set_body_string(body)
    } else {
        ResponseTemplate::new(status)
    };

    Mock::given(method_matcher(method))
        .and(path_matcher(path))
        .respond_with(template)
        .mount(server)
        .await;
}
