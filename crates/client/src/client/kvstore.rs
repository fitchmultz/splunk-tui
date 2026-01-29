//! KVStore API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Getting KVStore status information
//!
//! # What this module does NOT handle:
//! - KVStore collection management (not yet implemented)
//! - Low-level KVStore endpoint HTTP calls (in [`crate::endpoints::kvstore`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::KvStoreStatus;

impl SplunkClient {
    /// Get KVStore status information.
    pub async fn get_kvstore_status(&mut self) -> Result<KvStoreStatus> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_kvstore_status(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
