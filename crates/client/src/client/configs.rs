//! Configuration file management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing configuration files
//! - Listing configuration stanzas for a specific config file
//! - Retrieving specific configuration stanzas
//!
//! # What this module does NOT handle:
//! - Creating or modifying configuration stanzas (not yet implemented)
//! - Low-level config endpoint HTTP calls (in [`crate::endpoints::configs`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{ConfigFile, ConfigStanza};
use std::collections::HashMap;

impl SplunkClient {
    /// List all configuration files.
    ///
    /// Returns a list of supported configuration files with their titles
    /// and descriptions.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `ConfigFile` structs on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails.
    pub async fn list_config_files(&self) -> Result<Vec<ConfigFile>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_config_files(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List configuration stanzas for a specific config file.
    ///
    /// # Arguments
    ///
    /// * `config_file` - The config file name (e.g., "props", "transforms")
    /// * `count` - Maximum number of results to return (default: 30)
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `ConfigStanza` structs on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails or the response cannot be parsed.
    pub async fn list_config_stanzas(
        &self,
        config_file: &str,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<ConfigStanza>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_config_stanzas(
                &self.http,
                &self.base_url,
                &__token,
                config_file,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get a specific configuration stanza.
    ///
    /// # Arguments
    ///
    /// * `config_file` - The config file name (e.g., "props", "transforms")
    /// * `stanza_name` - The name of the stanza to retrieve
    ///
    /// # Returns
    ///
    /// A `Result` containing a `ConfigStanza` struct on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails or the stanza is not found.
    pub async fn get_config_stanza(
        &self,
        config_file: &str,
        stanza_name: &str,
    ) -> Result<ConfigStanza> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_config_stanza(
                &self.http,
                &self.base_url,
                &__token,
                config_file,
                stanza_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List stanzas across all supported config files (aggregated).
    ///
    /// This method queries all supported config files and aggregates the results
    /// into a HashMap mapping config file names to their stanzas.
    ///
    /// # Arguments
    ///
    /// * `count_per_file` - Maximum number of stanzas to return per config file
    ///
    /// # Returns
    ///
    /// A `Result` containing a HashMap of config file names to vectors of
    /// `ConfigStanza` structs on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if any request fails. Partial results are not returned.
    pub async fn list_all_config_stanzas(
        &self,
        count_per_file: Option<u64>,
    ) -> Result<HashMap<String, Vec<ConfigStanza>>> {
        let config_files = self.list_config_files().await?;
        let mut result = HashMap::new();

        for config_file in config_files {
            let stanzas = self
                .list_config_stanzas(&config_file.name, count_per_file, None)
                .await?;
            result.insert(config_file.name, stanzas);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests would require a mock server or integration test setup
    // For now, we just verify the module compiles correctly

    #[test]
    fn test_config_stanza_structure() {
        let stanza = ConfigStanza {
            name: "test_stanza".to_string(),
            config_file: "props".to_string(),
            settings: std::collections::HashMap::new(),
        };
        assert_eq!(stanza.name, "test_stanza");
        assert_eq!(stanza.config_file, "props");
        assert!(stanza.settings.is_empty());
    }

    #[test]
    fn test_config_file_structure() {
        let config_file = ConfigFile {
            name: "transforms".to_string(),
            title: "Transforms Configuration".to_string(),
            description: Some("Test description".to_string()),
        };
        assert_eq!(config_file.name, "transforms");
        assert_eq!(config_file.title, "Transforms Configuration");
        assert_eq!(
            config_file.description,
            Some("Test description".to_string())
        );
    }
}
