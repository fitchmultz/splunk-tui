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
        crate::retry_call!(
            self,
            __token,
            endpoints::list_audit_events(
                &self.http,
                &self.base_url,
                &__token,
                params,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get recent audit events from the last 24 hours.
    pub async fn get_recent_audit_events(&self, count: u64) -> Result<Vec<AuditEvent>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_recent_audit_events(
                &self.http,
                &self.base_url,
                &__token,
                count,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
