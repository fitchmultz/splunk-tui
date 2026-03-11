//! Audit event API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing audit events
//! - Getting recent audit events
//!
//! # What this module does NOT handle:
//! - Low-level audit endpoint HTTP calls (in [`crate::endpoints::audit`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::audit::{AuditEvent, ListAuditEventsParams};

impl SplunkClient {
    /// List audit events with optional filters.
    pub async fn list_audit_events(
        &self,
        params: &ListAuditEventsParams,
    ) -> Result<Vec<AuditEvent>> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("list_audit_events"),
            |__token| async move {
                endpoints::list_audit_events(
                    &self.http,
                    &self.base_url,
                    &__token,
                    params,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Get recent audit events from the last 24 hours.
    pub async fn get_recent_audit_events(&self, count: usize) -> Result<Vec<AuditEvent>> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation(
                "get_recent_audit_events",
            ),
            |__token| async move {
                endpoints::get_recent_audit_events(
                    &self.http,
                    &self.base_url,
                    &__token,
                    count,
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
