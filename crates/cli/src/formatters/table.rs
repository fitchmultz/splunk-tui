//! Table formatter implementation.
//!
//! Responsibilities:
//! - Format resources as tab-separated tables.
//! - Provide paginated variants for interactive use.
//!
//! Does NOT handle:
//! - Other output formats.
//! - File I/O.
//!
//! Invariants:
//! - Tables use tab-separation for consistent terminal alignment
//! - Empty collections display a clear "No X found" message

pub use self::imp::Pagination;
pub use self::imp::TableFormatter;

// Submodules containing individual format_* implementations
mod alerts;
mod apps;
mod cluster;
mod configs;
mod dashboards;
mod datamodels;
mod forwarders;
mod health;
mod hec;
mod imp;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod lookups;
mod macros;
mod profiles;
mod roles;
mod saved_searches;
mod search;
mod search_peers;
mod shc;
mod users;
mod workload;
