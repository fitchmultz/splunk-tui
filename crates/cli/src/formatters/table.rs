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
mod apps;
mod cluster;
mod health;
mod imp;
mod indexes;
mod jobs;
mod license;
mod list_all;
mod logs;
mod profiles;
mod saved_searches;
mod search;
mod users;
