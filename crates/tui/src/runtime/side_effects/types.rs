//! Shared types for side effect handlers.
//!
//! This module contains type aliases and shared definitions used across
//! all side effect handler submodules.

use splunk_client::SplunkClient;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared client wrapper for async tasks.
///
/// This type alias provides a thread-safe, shared reference to the SplunkClient
/// that can be passed to spawned async tasks. The Mutex ensures exclusive access
/// for API calls (required for session token refresh which needs &mut self).
pub type SharedClient = Arc<Mutex<SplunkClient>>;
