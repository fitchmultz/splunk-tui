//! CSV formatter implementation.
//!
//! Responsibilities:
//! - Format resources as RFC 4180 compliant CSV.
//! - Flatten nested JSON structures for tabular output.
//!
//! Does NOT handle:
//! - Other output formats.
//! - Table-style pagination.

pub use self::imp::CsvFormatter;

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
mod macros;
mod profiles;
mod roles;
mod saved_searches;
mod search;
mod search_peers;
mod users;
