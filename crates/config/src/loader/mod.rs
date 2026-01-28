//! Configuration loader for environment variables and files.
//!
//! Responsibilities:
//! - Load configuration from `.env` files, environment variables, and JSON profile files.
//! - Provide a builder-pattern `ConfigLoader` for hierarchical configuration merging.
//! - Enforce `DOTENV_DISABLED` gate to prevent accidental dotenv loading in tests.
//!
//! Does NOT handle:
//! - Persisting configuration changes back to disk (see `persistence.rs`).
//! - Interaction with system keyrings directly (delegated to `types.rs` via `resolve()`).
//!
//! Invariants / Assumptions:
//! - Environment variables take precedence over profile file values.
//! - `load_dotenv()` must be called explicitly to enable `.env` file loading.
//! - The `DOTENV_DISABLED` variable is checked before `dotenvy::dotenv()` is called.

mod builder;
mod defaults;
mod env;
mod error;
mod profile;

pub use builder::ConfigLoader;
pub use defaults::SearchDefaultConfig;
pub use env::env_var_or_none;
pub use error::ConfigError;
