//! Security-focused redaction wrapper for Action logging.
//!
//! This module provides `RedactedAction`, a wrapper type that implements
//! `Debug` to prevent sensitive payloads from being written to log files.
//! This is a critical security feature - always use `RedactedAction(&action)`
//! instead of `?action` when logging.
//!
//! # Security Invariants
//!
//! - All Action variants containing sensitive data MUST be handled explicitly
//! - Sensitive data includes: passwords, tokens, API responses, user names,
//!   profile names, error messages, and search queries (may contain PII, tokens)
//! - Search queries ARE considered sensitive (may contain PII, tokens, incident IDs)
//! - Non-sensitive simple variants fall through to default Debug
//!
//! # What This Module Does NOT Handle
//!
//! - Actual logging infrastructure (handled by tracing)
//! - Log filtering or redaction at other layers
//! - Encryption or secure storage of data
//!
//! # Example
//!
//! ```ignore
//! use splunk_config::SearchDefaults;
//! let action = Action::RunSearch {
//!     query: "SELECT * FROM users WHERE password='secret'".to_string(),
//!     search_defaults: SearchDefaults::default(),
//! };
//! tracing::info!("Handling action: {:?}", RedactedAction(&action));
//! // Logs: Handling action: RunSearch(<43 chars, hash=296582a1>)
//! ```

use crate::action::variants::Action;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Redact a query string for logging, showing only length and a short hash prefix.
/// This allows operators to correlate logs without exposing query content.
fn redact_query(query: &str) -> String {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    let hash = hasher.finish();
    format!("<{} chars, hash={:08x}>", query.len(), hash)
}

/// Redacted wrapper for Action that prevents sensitive payloads from being logged.
///
/// This wrapper implements `Debug` to replace sensitive string payloads with
/// size indicators (e.g., `<42 chars>`) while preserving non-sensitive
/// information for debugging purposes.
///
/// # Example
/// ```ignore
/// use splunk_config::SearchDefaults;
/// let action = Action::RunSearch {
///     query: "SELECT * FROM users WHERE password='secret'".to_string(),
///     search_defaults: SearchDefaults::default(),
/// };
/// tracing::info!("Handling action: {:?}", RedactedAction(&action));
/// // Logs: Handling action: RunSearch(<43 chars, hash=296582a1>)
/// ```
pub struct RedactedAction<'a>(pub &'a Action);

impl std::fmt::Debug for RedactedAction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Action::RunSearch { query, .. } => {
                write!(f, "RunSearch({})", redact_query(query))
            }
            Action::CopyToClipboard(text) => {
                write!(f, "CopyToClipboard(<{} chars>)", text.len())
            }
            Action::ExportData(data, path, format) => {
                let data_size = data.to_string().len();
                write!(
                    f,
                    "ExportData(<{} bytes>, {:?}, {:?})",
                    data_size, path, format
                )
            }
            Action::Notify(level, message) => {
                write!(f, "Notify({:?}, <{} chars>)", level, message.len())
            }
            Action::CancelJob(sid) => write!(f, "CancelJob({})", sid),
            Action::DeleteJob(sid) => write!(f, "DeleteJob({})", sid),
            Action::CancelJobsBatch(sids) => {
                write!(f, "CancelJobsBatch([{} job(s)])", sids.len())
            }
            Action::DeleteJobsBatch(sids) => {
                write!(f, "DeleteJobsBatch([{} job(s)])", sids.len())
            }
            Action::EnableApp(name) => write!(f, "EnableApp({})", name),
            Action::DisableApp(name) => write!(f, "DisableApp({})", name),
            Action::SearchInput(c) => write!(f, "SearchInput({:?})", c),

            // Search-related actions with sensitive data
            Action::SearchStarted(query) => {
                write!(f, "SearchStarted({})", redact_query(query))
            }
            Action::SearchComplete(result) => match result {
                Ok((results, sid, total)) => {
                    write!(
                        f,
                        "SearchComplete(<{} results>, sid={}, total={:?})",
                        results.len(),
                        sid,
                        total
                    )
                }
                Err(_) => write!(f, "SearchComplete(<error>)"),
            },
            Action::MoreSearchResultsLoaded(result) => match result {
                Ok((results, offset, total)) => {
                    write!(
                        f,
                        "MoreSearchResultsLoaded(<{} results>, offset={}, total={:?})",
                        results.len(),
                        offset,
                        total
                    )
                }
                Err(_) => write!(f, "MoreSearchResultsLoaded(<error>)"),
            },

            // Data-loaded actions - show item count, not content
            Action::IndexesLoaded(result) => match result {
                Ok(items) => write!(f, "IndexesLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "IndexesLoaded(<error>)"),
            },
            Action::JobsLoaded(result) => match result {
                Ok(items) => write!(f, "JobsLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "JobsLoaded(<error>)"),
            },
            Action::ClusterInfoLoaded(result) => match result {
                Ok(_) => write!(f, "ClusterInfoLoaded(<data>)"),
                Err(_) => write!(f, "ClusterInfoLoaded(<error>)"),
            },
            Action::HealthLoaded(result) => match result.as_ref() {
                Ok(_) => write!(f, "HealthLoaded(<data>)"),
                Err(_) => write!(f, "HealthLoaded(<error>)"),
            },
            Action::SavedSearchesLoaded(result) => match result {
                Ok(items) => write!(f, "SavedSearchesLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "SavedSearchesLoaded(<error>)"),
            },
            Action::InternalLogsLoaded(result) => match result {
                Ok(items) => write!(f, "InternalLogsLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "InternalLogsLoaded(<error>)"),
            },
            Action::AppsLoaded(result) => match result {
                Ok(items) => write!(f, "AppsLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "AppsLoaded(<error>)"),
            },
            Action::UsersLoaded(result) => match result {
                Ok(items) => write!(f, "UsersLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "UsersLoaded(<error>)"),
            },
            Action::ClusterPeersLoaded(result) => match result {
                Ok(items) => write!(f, "ClusterPeersLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "ClusterPeersLoaded(<error>)"),
            },
            Action::HealthStatusLoaded(result) => match result {
                Ok(_) => write!(f, "HealthStatusLoaded(<data>)"),
                Err(_) => write!(f, "HealthStatusLoaded(<error>)"),
            },

            // Profile-related actions
            Action::OpenProfileSelectorWithList(profiles) => {
                write!(
                    f,
                    "OpenProfileSelectorWithList(<{} profiles>)",
                    profiles.len()
                )
            }
            Action::ProfileSwitchResult(result) => match result {
                Ok(_) => write!(f, "ProfileSwitchResult(Ok)"),
                Err(_) => write!(f, "ProfileSwitchResult(Err)"),
            },
            Action::ProfileSelected(_) => write!(f, "ProfileSelected(<redacted>)"),

            // Settings-loaded action contains search history that may have sensitive queries
            Action::SettingsLoaded(_) => write!(f, "SettingsLoaded(<redacted>)"),

            // Error details may contain sensitive URLs, queries, or response data
            Action::ShowErrorDetails(_) => write!(f, "ShowErrorDetails(<redacted>)"),
            Action::ShowErrorDetailsFromCurrent => write!(f, "ShowErrorDetailsFromCurrent"),

            // Non-sensitive simple actions - fall through to default Debug
            other => write!(f, "{:?}", other),
        }
    }
}
