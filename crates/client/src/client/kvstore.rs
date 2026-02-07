//! KVStore API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Getting KVStore status information
//! - KVStore collection management (list, create, modify, delete)
//! - KVStore collection data access (list, insert, delete records)
//!
//! # What this module does NOT handle:
//! - Low-level KVStore endpoint HTTP calls (in [`crate::endpoints::kvstore`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{
    CreateCollectionParams, KvStoreCollection, KvStoreRecord, KvStoreStatus, ModifyCollectionParams,
};

impl SplunkClient {
    /// Get KVStore status information.
    pub async fn get_kvstore_status(&self) -> Result<KvStoreStatus> {
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

    /// List all KVStore collections.
    pub async fn list_collections(
        &self,
        app: Option<&str>,
        owner: Option<&str>,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<KvStoreCollection>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_collections(
                &self.http,
                &self.base_url,
                &__token,
                app,
                owner,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Create a new KVStore collection.
    pub async fn create_collection(
        &self,
        params: &CreateCollectionParams,
    ) -> Result<KvStoreCollection> {
        crate::retry_call!(
            self,
            __token,
            endpoints::create_collection(
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

    /// Modify an existing KVStore collection.
    pub async fn modify_collection(
        &self,
        name: &str,
        app: &str,
        owner: &str,
        params: &ModifyCollectionParams,
    ) -> Result<KvStoreCollection> {
        crate::retry_call!(
            self,
            __token,
            endpoints::modify_collection(
                &self.http,
                &self.base_url,
                &__token,
                name,
                app,
                owner,
                params,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Delete a KVStore collection.
    pub async fn delete_collection(&self, name: &str, app: &str, owner: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::delete_collection(
                &self.http,
                &self.base_url,
                &__token,
                name,
                app,
                owner,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List records in a collection.
    pub async fn list_collection_records(
        &self,
        collection_name: &str,
        app: &str,
        owner: &str,
        query: Option<&str>,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<KvStoreRecord>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_collection_records(
                &self.http,
                &self.base_url,
                &__token,
                collection_name,
                app,
                owner,
                query,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Insert a record into a collection.
    pub async fn insert_collection_record(
        &self,
        collection_name: &str,
        app: &str,
        owner: &str,
        record: &serde_json::Value,
    ) -> Result<KvStoreRecord> {
        crate::retry_call!(
            self,
            __token,
            endpoints::insert_collection_record(
                &self.http,
                &self.base_url,
                &__token,
                collection_name,
                app,
                owner,
                record,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Delete a record from a collection.
    pub async fn delete_collection_record(
        &self,
        collection_name: &str,
        app: &str,
        owner: &str,
        record_key: &str,
    ) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::delete_collection_record(
                &self.http,
                &self.base_url,
                &__token,
                collection_name,
                app,
                owner,
                record_key,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
