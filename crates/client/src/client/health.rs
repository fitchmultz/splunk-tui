//! Health check aggregation operations.
//!
//! This module provides shared health check logic used by both CLI and TUI,
//! eliminating duplication of the health check aggregation pattern.

use crate::client::SplunkClient;
use crate::error::Result;
use crate::models::HealthCheckOutput;

/// Result of a health check aggregation.
///
/// Individual health components may fail without failing the entire check.
/// The `server_info` field is always present on success; other fields are optional.
#[derive(Debug)]
pub struct AggregatedHealth {
    /// The aggregated health check output containing all collected data.
    pub output: HealthCheckOutput,
    /// Errors from individual health checks that failed but didn't abort the aggregation.
    /// Each tuple contains (endpoint_name, error).
    pub partial_errors: Vec<(String, crate::error::ClientError)>,
}

impl SplunkClient {
    /// Perform a comprehensive health check by aggregating multiple endpoints.
    ///
    /// This method collects health information from:
    /// - Server info (always fetched, required for basic health)
    /// - Splunkd health endpoint
    /// - License usage
    /// - KVStore status
    /// - Log parsing health
    ///
    /// # Returns
    ///
    /// Returns `Ok(AggregatedHealth)` if server_info can be fetched.
    /// Returns `Err` only if server_info fails (indicating the server is unreachable).
    /// Other endpoints may fail and will be recorded in `partial_errors`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let health = client.check_health_aggregate().await?;
    /// println!("Server version: {:?}", health.output.server_info?.version);
    /// for (endpoint, err) in &health.partial_errors {
    ///     eprintln!("Warning: {} check failed: {}", endpoint, err);
    /// }
    /// ```
    pub async fn check_health_aggregate(&mut self) -> Result<AggregatedHealth> {
        // Server info is required - if this fails, the whole check fails
        let server_info = self.get_server_info().await?;

        let mut output = HealthCheckOutput {
            server_info: Some(server_info),
            splunkd_health: None,
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };
        let mut partial_errors = Vec::new();

        // Collect optional health data - failures don't abort the aggregation
        match self.get_health().await {
            Ok(health) => output.splunkd_health = Some(health),
            Err(e) => partial_errors.push(("splunkd_health".to_string(), e)),
        }

        match self.get_license_usage().await {
            Ok(usage) => output.license_usage = Some(usage),
            Err(e) => partial_errors.push(("license_usage".to_string(), e)),
        }

        match self.get_kvstore_status().await {
            Ok(status) => output.kvstore_status = Some(status),
            Err(e) => partial_errors.push(("kvstore_status".to_string(), e)),
        }

        match self.check_log_parsing_health().await {
            Ok(log_health) => output.log_parsing_health = Some(log_health),
            Err(e) => partial_errors.push(("log_parsing_health".to_string(), e)),
        }

        Ok(AggregatedHealth {
            output,
            partial_errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ServerInfo, SplunkHealth};

    #[test]
    fn test_aggregated_health_structure() {
        let output = HealthCheckOutput {
            server_info: Some(ServerInfo {
                server_name: "test".to_string(),
                version: "9.0.0".to_string(),
                build: "abc123".to_string(),
                mode: Some("standalone".to_string()),
                server_roles: vec!["search_head".to_string()],
                os_name: Some("Linux".to_string()),
            }),
            splunkd_health: None,
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };

        let aggregated = AggregatedHealth {
            output,
            partial_errors: vec![],
        };

        assert!(aggregated.output.server_info.is_some());
        assert_eq!(aggregated.output.server_info.unwrap().server_name, "test");
        assert!(aggregated.partial_errors.is_empty());
    }

    #[test]
    fn test_aggregated_health_with_partial_errors() {
        let output = HealthCheckOutput {
            server_info: Some(ServerInfo {
                server_name: "test".to_string(),
                version: "9.0.0".to_string(),
                build: "abc123".to_string(),
                mode: Some("standalone".to_string()),
                server_roles: vec!["search_head".to_string()],
                os_name: Some("Linux".to_string()),
            }),
            splunkd_health: Some(SplunkHealth {
                health: "green".to_string(),
                features: std::collections::HashMap::new(),
            }),
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };

        let partial_errors = vec![
            (
                "license_usage".to_string(),
                crate::error::ClientError::ApiError {
                    status: 503,
                    url: "/services/licenser/usage".to_string(),
                    message: "License manager unavailable".to_string(),
                    request_id: None,
                },
            ),
            (
                "kvstore_status".to_string(),
                crate::error::ClientError::ApiError {
                    status: 503,
                    url: "/services/kvstore/status".to_string(),
                    message: "KVStore not ready".to_string(),
                    request_id: Some("req-123".to_string()),
                },
            ),
        ];

        let aggregated = AggregatedHealth {
            output,
            partial_errors,
        };

        assert_eq!(aggregated.partial_errors.len(), 2);
        assert!(aggregated.output.splunkd_health.is_some());
        assert!(aggregated.output.license_usage.is_none());
    }
}
