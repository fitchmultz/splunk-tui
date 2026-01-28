//! Path helpers for configuration file locations.
//!
//! Responsibilities:
//! - Determine standard and legacy configuration file paths.
//! - Use `directories` crate for platform-appropriate paths.
//!
//! Does NOT handle:
//! - File I/O operations.
//! - Migration logic.
//! - Profile management.

use std::path::PathBuf;

use anyhow::Context;

/// Returns the default path to the configuration file.
///
/// This path is the **documented** config location:
/// - Linux/macOS: `~/.config/splunk-tui/config.json`
/// - Windows: `%AppData%\splunk-tui\config.json`
pub(crate) fn default_config_path() -> Result<PathBuf, anyhow::Error> {
    let proj_dirs = directories::ProjectDirs::from("", "", "splunk-tui")
        .context("Failed to determine project directories")?;

    Ok(proj_dirs.config_dir().join("config.json"))
}

/// Returns the legacy path to the configuration file used by older versions.
///
/// Legacy implementation used:
/// `ProjectDirs::from("com", "splunk-tui", "splunk-tui")`
/// which produced a redundant directory segment like:
/// `.../splunk-tui/splunk-tui/config.json`
pub(crate) fn legacy_config_path() -> Result<PathBuf, anyhow::Error> {
    let proj_dirs = directories::ProjectDirs::from("com", "splunk-tui", "splunk-tui")
        .context("Failed to determine legacy project directories")?;

    Ok(proj_dirs.config_dir().join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_path_matches_expected_project_dirs() {
        let expected = directories::ProjectDirs::from("", "", "splunk-tui")
            .unwrap()
            .config_dir()
            .join("config.json");

        let actual = default_config_path().unwrap();
        assert_eq!(actual, expected);

        // Ensure we did not keep the legacy redundant path segment.
        let s = actual.to_string_lossy();
        assert!(!s.contains("splunk-tui/splunk-tui"));
        assert!(!s.contains("splunk-tui\\splunk-tui"));
    }
}
