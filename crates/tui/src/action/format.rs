//! Export format types for TUI data export operations.
//!
//! This module defines the supported export formats for search results and
//! other data in the TUI. The actual export logic resides in the UI layer,
//! not in this module.
//!
//! # Supported Formats
//!
//! - `Json`: JSON format with proper serialization
//! - `Csv`: CSV format for spreadsheet applications
//! - `Ndjson`: NDJSON format for streaming workflows
//! - `Yaml`: YAML format for human-readable configuration exports
//! - `Markdown`: Markdown format for documentation and reports
//!
//! # What This Module Does NOT Handle
//!
//! - File I/O operations (handled by the UI layer)
//! - Data serialization (handled by `serde_json`)
//! - Export progress reporting (handled via `Action::Progress`)

/// Supported export formats for search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// NDJSON format (newline-delimited JSON)
    Ndjson,
    /// YAML format
    Yaml,
    /// Markdown format
    Markdown,
}
