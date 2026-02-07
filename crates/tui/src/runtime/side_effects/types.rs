//! Shared types for side effect handlers.
//!
//! This module contains type aliases and shared definitions used across
//! all side effect handler submodules.

use splunk_client::SplunkClient;
use std::sync::Arc;

/// Shared client wrapper for async tasks.
///
/// This type alias provides a thread-safe, shared reference to the SplunkClient
/// that can be passed to spawned async tasks. Since SplunkClient now uses
/// interior mutability for session management, no Mutex is required.
///
/// Concurrency is controlled at the call site using semaphores where needed.
pub type SharedClient = Arc<SplunkClient>;
