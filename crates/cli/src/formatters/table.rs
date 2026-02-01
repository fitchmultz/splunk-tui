//! Table formatter implementation.
//!
//! Responsibilities:
//! - Format resources as tab-separated tables.
//! - Provide paginated variants for interactive use.
//!
//! Does NOT handle:
//! - Other output formats.
//! - File I/O.

pub use self::imp::Pagination;
pub use self::imp::TableFormatter;

// Submodules containing individual format_* implementations
mod alerts;
mod apps;
mod cluster;
mod configs;
mod forwarders;
mod health;
mod hec;
mod imp;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod list_all;
mod logs;
mod lookups;
mod profiles;
mod roles;
mod saved_searches;
mod search;
mod search_peers;
mod users;
