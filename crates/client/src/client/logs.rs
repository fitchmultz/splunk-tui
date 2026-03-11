//! Log parsing and internal log API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Checking log parsing health
//! - Retrieving internal logs
//!
//! # What this module does NOT handle:
//! - Log forwarding configuration (not yet implemented)
//! - Low-level log endpoint HTTP calls (in [`crate::endpoints::logs`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{LogEntry, LogParsingHealth};

impl SplunkClient {
    /// Check log parsing health by searching for parsing errors in internal logs.
    ///
    /// This method searches the `_internal` index for parsing-related errors
    /// from specific components (TuningParser, DateParserVerbose, Parser) and
    /// returns structured results about any issues found.
    pub async fn check_log_parsing_health(&self) -> Result<LogParsingHealth> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation(
                "check_log_parsing_health",
            ),
            |__token| async move {
                endpoints::check_log_parsing_health(
                    &self.http,
                    &self.base_url,
                    &__token,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Get internal logs from Splunk.
    pub async fn get_internal_logs(
        &self,
        count: usize,
        earliest: Option<&str>,
    ) -> Result<Vec<LogEntry>> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_internal_logs"),
            |__token| async move {
                endpoints::get_internal_logs(
                    &self.http,
                    &self.base_url,
                    &__token,
                    count,
                    earliest,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }
}
