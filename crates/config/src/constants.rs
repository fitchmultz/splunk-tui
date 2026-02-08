//! Centralized constants for the Splunk TUI workspace.
//!
//! This module contains default values used across crates to avoid
//! magic number duplication and improve maintainability.

// =============================================================================
// Connection & Timeout Defaults
// =============================================================================

/// Default HTTP request timeout in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default session time-to-live in seconds (1 hour).
pub const DEFAULT_SESSION_TTL_SECS: u64 = 3600;

/// Default buffer time before session expiry to proactively refresh tokens.
/// This prevents race conditions where a token expires during an API call.
pub const DEFAULT_EXPIRY_BUFFER_SECS: u64 = 60;

/// Default health check polling interval in seconds.
pub const DEFAULT_HEALTH_CHECK_INTERVAL_SECS: u64 = 60;

// =============================================================================
// Timeout Configuration Bounds
// =============================================================================

/// Maximum allowed connection timeout in seconds (1 hour).
pub const MAX_TIMEOUT_SECS: u64 = 3600;

/// Maximum allowed session TTL in seconds (24 hours).
pub const MAX_SESSION_TTL_SECS: u64 = 86400;

/// Maximum allowed health check interval in seconds (1 hour).
pub const MAX_HEALTH_CHECK_INTERVAL_SECS: u64 = 3600;

/// Default Splunk management port.
pub const DEFAULT_SPLUNK_PORT: u16 = 8089;

/// Default maximum number of HTTP redirects to follow.
pub const DEFAULT_MAX_REDIRECTS: usize = 5;

/// Default maximum number of retries for failed requests.
pub const DEFAULT_MAX_RETRIES: usize = 3;

// =============================================================================
// Search & Polling Defaults
// =============================================================================

/// Default polling interval for job status checks in milliseconds.
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 500;

/// Default maximum time to wait for search job completion in seconds.
pub const DEFAULT_MAX_WAIT_SECS: u64 = 300;

/// Default maximum number of search results to return.
pub const DEFAULT_MAX_RESULTS: usize = 1000;

// =============================================================================
// TUI/UI Defaults
// =============================================================================

/// Default channel capacity for action messages.
pub const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// Default UI tick interval for animations in milliseconds.
pub const DEFAULT_UI_TICK_MS: u64 = 250;

/// Default data refresh interval in seconds.
pub const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 5;

/// Default scroll threshold for triggering pagination (items from end).
pub const DEFAULT_SCROLL_THRESHOLD: usize = 10;

// =============================================================================
// List Pagination Defaults (for TUI list screens)
// =============================================================================

/// Default page size for list screens (indexes, jobs, apps, users).
pub const DEFAULT_LIST_PAGE_SIZE: usize = 100;

/// Default maximum items to load for list screens (safety limit).
pub const DEFAULT_LIST_MAX_ITEMS: usize = 1000;

/// Default maximum number of search history items to retain.
pub const DEFAULT_HISTORY_MAX_ITEMS: usize = 50;

/// Default maximum characters for clipboard preview in toasts.
pub const DEFAULT_CLIPBOARD_PREVIEW_CHARS: usize = 30;

// =============================================================================
// CLI/Formatting Defaults
// =============================================================================

/// Default license usage percentage threshold for alerting.
pub const DEFAULT_LICENSE_ALERT_PCT: f64 = 90.0;

// =============================================================================
// Internal Logs Defaults
// =============================================================================

/// Default number of internal log entries to fetch per query.
pub const DEFAULT_INTERNAL_LOGS_COUNT: usize = 100;

/// Default earliest time for internal logs queries (last 15 minutes).
pub const DEFAULT_INTERNAL_LOGS_EARLIEST_TIME: &str = "-15m";

// =============================================================================
// Test Timing Constants
// =============================================================================

/// Delay duration for migration tests to ensure different timestamps (milliseconds).
pub const MIGRATION_DELAY_MS: u64 = 1100;

// =============================================================================
// TUI Toast Notification Defaults
// =============================================================================

/// Default TTL for short-lived toasts (info, success, warning) in seconds.
pub const DEFAULT_TOAST_TTL_SHORT_SECS: u64 = 5;

/// Default TTL for long-lived toasts (errors) in seconds.
pub const DEFAULT_TOAST_TTL_LONG_SECS: u64 = 10;

/// Maximum number of toasts to display simultaneously.
pub const DEFAULT_MAX_TOASTS: usize = 5;

/// Height of each toast in terminal lines (includes borders and content).
pub const DEFAULT_TOAST_HEIGHT: u16 = 4;

/// Width of toast notifications in terminal columns.
pub const DEFAULT_TOAST_WIDTH: u16 = 60;

/// Maximum number of lines to display in a toast message.
pub const DEFAULT_TOAST_MAX_LINES: usize = 2;

/// Padding applied to toast content width calculations (left + right margins).
pub const DEFAULT_TOAST_PADDING: u16 = 4;

// =============================================================================
// Input Validation Defaults
// =============================================================================

/// Debounce delay for SPL validation in milliseconds.
pub const DEFAULT_VALIDATION_DEBOUNCE_MS: u64 = 500;

// =============================================================================
// CLI Defaults
// =============================================================================

/// Default poll interval for tail mode in seconds.
pub const DEFAULT_LOGS_TAIL_POLL_INTERVAL_SECS: u64 = 2;
