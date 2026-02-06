//! Async side effect handlers for TUI actions.
//!
//! Responsibilities:
//! - Handle async API calls triggered by user actions.
//! - Spawn background tasks for data fetching to avoid blocking the UI.
//! - Send results back via the action channel for state updates.
//!
//! Does NOT handle:
//! - Direct application state modification (sends actions to do that).
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
//!    async operations (like acquiring a mutex) can cause frame drops if they
//!    contend with the render thread.
//!
//! 2. **Consistent Error Boundaries**: Each spawned task is an isolated failure
//!    domain. A panic in one API call handler won't crash the entire application.
//!
//! 3. **Cancellation Safety**: Tasks can be dropped without cleanup concerns
//!    (the client mutex is released on drop, and API calls are stateless).
//!
//! ## The Mutex Bottleneck
//!
//! All API calls share a single `Arc<Mutex<SplunkClient>>`. This means:
//! - **API calls are serialized** regardless of how many tasks are spawned
//! - Multiple concurrent tasks simply queue for the client lock
//! - Task spawn overhead is negligible compared to network I/O latency
//!
//! This is a deliberate trade-off: the SplunkClient requires `&mut self` for
//! session token refresh, so true parallel API calls would require significant
//! architectural changes (e.g., connection pooling or token refresh decoupling).
//!
//! ## Sequential Operations
//!
//! Some operations intentionally sequential:
//!
//! - **Health checks** (`LoadHealth`): 5 API calls run sequentially within one
//!   spawned task due to the `&mut self` requirement. Parallelizing would require
//!   either spawning 5 separate tasks (each waiting for the lock) or refactoring
//!   the client to support concurrent access.
//!
//! - **Batch operations** (`CancelJobsBatch`, `DeleteJobsBatch`): Jobs are
//!   processed sequentially to avoid overwhelming the Splunk API and to provide
//!   clear per-job error reporting.
//!
//! ## Performance Considerations
//!
//! Tokio task spawning has minimal overhead (~microseconds). Given that:
//! - Network I/O dominates latency (milliseconds to seconds)
//! - The client mutex serializes actual API calls
//! - No measured bottleneck exists in task scheduling
//!
//! The current pattern is not a performance concern. Optimization would only be
//! warranted if profiling shows significant time in task scheduling overhead.
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
//! 3. **Parallel health checks**: Spawn separate tasks per health endpoint
//!    (each would still serialize on the client lock, but they'd pipeline better).
//!
//! 4. **Parallel batch operations**: Use `futures::future::join_all` for batch
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
pub use types::SharedClient;
