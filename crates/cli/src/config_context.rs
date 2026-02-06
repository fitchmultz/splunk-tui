//! Configuration context for command execution.
//!
//! Responsibilities:
//! - Distinguish between real and placeholder configs at compile time.
//! - Provide type-safe extraction of config for commands that need it.
//!
//! Does NOT handle:
//! - Configuration loading (done in `main()`).
//! - CLI argument definitions (see `args` module).
//!
//! Invariants:
//! - Placeholder configs cannot be used for actual Splunk API connections
//! - Real configs are validated before command execution

use splunk_config::SearchDefaultConfig;

/// Context for command execution, distinguishing between real and placeholder configs.
///
/// This enum provides compile-time guarantees that placeholder configs (used for
/// config management commands and multi-profile operations) cannot be accidentally
/// used for actual Splunk API connections.
pub(crate) enum ConfigCommandContext {
    /// A real, validated config loaded from profiles/environment/CLI args.
    /// Used for actual Splunk API operations.
    /// Includes search defaults for applying env var overrides to search parameters.
    Real(splunk_config::Config, SearchDefaultConfig),
    /// A placeholder config for commands that don't need real connection details.
    /// Only valid for Config commands and multi-profile ListAll operations.
    Placeholder,
}

impl ConfigCommandContext {
    /// Extract the real config, failing if this is a placeholder.
    ///
    /// Use this for commands that require actual connection details.
    pub(crate) fn into_real_config(self) -> anyhow::Result<splunk_config::Config> {
        match self {
            ConfigCommandContext::Real(config, _) => Ok(config),
            ConfigCommandContext::Placeholder => {
                anyhow::bail!(
                    "Internal error: attempted to use placeholder config for an operation requiring real connection details"
                )
            }
        }
    }

    /// Extract both the real config and search defaults, failing if this is a placeholder.
    ///
    /// Use this for commands that require actual connection details and search defaults.
    pub(crate) fn into_real_config_with_search_defaults(
        self,
    ) -> anyhow::Result<(splunk_config::Config, SearchDefaultConfig)> {
        match self {
            ConfigCommandContext::Real(config, search_defaults) => Ok((config, search_defaults)),
            ConfigCommandContext::Placeholder => {
                anyhow::bail!(
                    "Internal error: attempted to use placeholder config for an operation requiring real connection details"
                )
            }
        }
    }
}
