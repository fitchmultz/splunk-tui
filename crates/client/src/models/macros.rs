//! Search macro models for Splunk REST API.
//!
//! Responsibilities:
//! - Define structs for macro data from /services/admin/macros endpoint.
//! - Provide serialization/deserialization via serde.
//!
//! Non-responsibilities:
//! - Does not handle HTTP requests (see endpoints module).
//! - Does not contain business logic (see client module).

use serde::{Deserialize, Serialize};

/// A search macro definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Macro {
    /// Macro name (e.g., "my_macro" or "my_macro(2)" for parameterized)
    #[serde(default)]
    pub name: String,
    /// The SPL snippet or eval expression
    pub definition: String,
    /// Comma-separated argument names for parameterized macros
    #[serde(default)]
    pub args: Option<String>,
    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,
    /// Whether the macro is disabled
    #[serde(default)]
    pub disabled: bool,
    /// If true, macro is an eval expression (not SPL)
    #[serde(default)]
    pub iseval: bool,
    /// Optional validation expression
    #[serde(default)]
    pub validation: Option<String>,
    /// Error message shown if validation fails
    #[serde(default)]
    pub errormsg: Option<String>,
}

/// Wrapper for a single macro entry in list responses.
#[derive(Debug, Clone, Deserialize)]
pub struct MacroEntry {
    pub name: String,
    pub content: Macro,
}

/// Response from listing macros.
#[derive(Debug, Clone, Deserialize)]
pub struct MacroListResponse {
    #[serde(default)]
    pub entry: Vec<MacroEntry>,
}
