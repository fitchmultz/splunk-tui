//! Async side effect handlers for TUI actions.
//!
//! Responsibilities:
//! - Handle async API calls triggered by user actions.
//! - Spawn background tasks for data fetching to avoid blocking the UI.
//! - Send results back via the action channel for state updates.
//!
//! Does NOT handle:
//! - Direct application state modification (sends actions for that).
//! - UI rendering or terminal management.
//! - Configuration loading or persistence.
//!
//! Invariants:
//! - All API calls are spawned as separate tokio tasks.
//! - Results are always sent back via the action channel.
//! - Loading state is set before API calls and cleared after.
//!
//! # Design Rationale: Task Spawning Pattern
//!
//! This module uses `tokio::spawn` for all async operations (21 handlers as of
//! this writing). This design is intentional and addresses specific constraints:
//!
//! ## Why Spawn Tasks?
//!
//! 1. **UI Responsiveness**: The TUI event loop must never block. Even brief
//!    async operations can cause frame drops if they contend with the render thread.
//!
//! 2. **Consistent Error Boundaries**: Each spawned task is an isolated failure
//!    domain. A panic in one API call handler won't crash the entire application.
//!
//! 3. **Cancellation Safety**: Tasks can be dropped without cleanup concerns
//!    (API calls are stateless and SplunkClient uses interior mutability for
//!    session management).
//!
//! ## Concurrent API Access
//!
//! API calls share a single `Arc<SplunkClient>`. The client uses interior
//! mutability (via `RwLock` and singleflight pattern) for session token refresh,
//! allowing multiple concurrent API calls without requiring `&mut self`.
//!
//! Benefits:
//! - **True parallelism**: Multiple API calls can execute simultaneously
//! - **Singleflight refresh**: Only one login request is made even with concurrent
//!   callers that need token refresh
//! - **Better performance**: Health checks and list-all operations run concurrently
//!
//! ## Sequential Operations
//!
//! Some operations are intentionally sequential:
//!
//! - **Batch operations** (`CancelJobsBatch`, `DeleteJobsBatch`): Jobs are
//!   processed sequentially to avoid overwhelming the Splunk API and to provide
//!   clear per-job error reporting.
//!
//! ## Performance Considerations
//!
//! Tokio task spawning has minimal overhead (~microseconds). Given that:
//! - Network I/O dominates latency (milliseconds to seconds)
//! - The client supports concurrent API calls via `&self` methods
//! - No measured bottleneck exists in task scheduling
//!
//! The current pattern is not a performance concern. The concurrent client
//! access enables faster health aggregation and list-all operations.
//!
//! ## Future Optimization Paths
//!
//! If performance data indicates a need:
//!
//! 1. **Semaphore-based limiting**: Add a `tokio::sync::Semaphore` to cap
//!    concurrent spawned tasks (prevents unbounded memory growth under load).
//!
//! 2. **Non-API operations**: `SwitchToSettings`, `ExportData`, and
//!    `OpenProfileSwitcher` don't make API calls and could run without spawn.
//!
//! 3. **Parallel batch operations**: Use `futures::future::join_all` for batch
//!    job operations (with rate limiting to avoid API throttling).

// Core types
mod types;

// Action dispatcher
mod dispatcher;

// Domain-specific handlers
mod alerts;
mod apps;
mod audit;
mod cluster;
mod configs;
mod dashboards;
mod datamodels;
mod export;
mod forwarders;
mod health;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod lookups;
mod macros;
mod multi_instance;
mod overview;
mod overview_fetch;
mod profiles;
mod roles;
mod search_peers;
mod searches;
mod shc;
mod users;
mod workload;

// Re-export public API
pub use dispatcher::handle_side_effects;
pub use types::{SharedClient, TaskTracker};
