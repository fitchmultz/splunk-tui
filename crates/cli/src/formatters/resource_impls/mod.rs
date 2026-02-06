//! ResourceDisplay implementations for client models.
//!
//! This module provides ResourceDisplay implementations that allow
//! all formatters to automatically support new resource types.
//!
//! Adding a new resource type:
//! 1. Implement `ResourceDisplay` for the type in a new file (e.g., `indexes.rs`)
//! 2. Add the module declaration below
//! 3. Use the formatter macros in each formatter implementation
//!
//! # Example
//! ```ignore
//! // In resource_impls/indexes.rs:
//! impl ResourceDisplay for Index {
//!     fn headers(_detailed: bool) -> Vec<&'static str> {
//!         vec!["Name", "Size MB", "Events"]
//!     }
//!     // ... other methods
//! }
//!
//! // In csv/imp.rs:
//! impl_csv_formatter! {
//!     format_indexes: &[Index] => indexes,
//! }
//! ```

pub mod apps;
pub mod users;
