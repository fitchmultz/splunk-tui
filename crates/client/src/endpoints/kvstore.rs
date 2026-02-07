//! KVStore endpoints for status and collection management.

use reqwest::{Client, Url};

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{
    CollectionListResponse, CreateCollectionParams, KvStoreCollection, KvStoreMember,
    KvStoreRecord, KvStoreReplicationStatus, KvStoreStatus, ModifyCollectionParams,
};
use crate::name_merge::attach_entry_name;

/// Get KVStore status.
pub async fn get_kvstore_status(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<KvStoreStatus> {
    let url = format!("{}/services/kvstore/status", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/kvstore/status",
        "GET",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract content from the first entry
    let content = resp
        .get("entry")
        .and_then(|e| e.get(0))
        .and_then(|e| e.get("content"))
        .ok_or_else(|| {
            ClientError::InvalidResponse(
                "Missing entry content in KVStore status response".to_string(),
            )
        })?;

    // Splunk can return different KVStore schemas depending on mode/version:
    // - Clustered: { currentMember: {...}, replicationStatus: {...} }
    // - Standalone: { current: {...}, members: {...} }
    if content.get("currentMember").is_some() {
        return serde_json::from_value(content.clone()).map_err(|e| {
            ClientError::InvalidResponse(format!("Failed to parse KVStore status: {}", e))
        });
    }

    let current = content.get("current").ok_or_else(|| {
        ClientError::InvalidResponse(
            "Missing current/currentMember in KVStore status response".to_string(),
        )
    })?;

    let guid = current
        .get("guid")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let replica_set = current
        .get("replicaSet")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let port = current
        .get("port")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        .try_into()
        .unwrap_or(0);

    let status = current
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let replication_status = current
        .get("replicationStatus")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let status = if replication_status.is_empty() {
        status.to_string()
    } else {
        format!("{status} ({replication_status})")
    };

    let host = content
        .get("members")
        .and_then(|m| m.as_object())
        .and_then(|obj| obj.values().next())
        .and_then(|v| v.get("hostAndPort"))
        .and_then(|v| v.as_str())
        .and_then(|hp| hp.split(':').next())
        .unwrap_or("localhost")
        .to_string();

    Ok(KvStoreStatus {
        current_member: KvStoreMember {
            guid,
            host,
            port,
            replica_set,
            status,
        },
        // Standalone response doesn't expose oplog sizing; default to 0.
        replication_status: KvStoreReplicationStatus {
            oplog_size: 0,
            oplog_used: 0.0,
        },
    })
}

/// List all KVStore collections.
#[allow(clippy::too_many_arguments)]
pub async fn list_collections(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    app: Option<&str>,
    owner: Option<&str>,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<KvStoreCollection>> {
    let app = app.unwrap_or("-");
    let owner = owner.unwrap_or("nobody");
    let url = format!(
        "{}/servicesNS/{}/{}/storage/collections",
        base_url, owner, app
    );

    let mut query_params: Vec<(String, String)> = vec![
        ("output_mode".to_string(), "json".to_string()),
        ("count".to_string(), count.unwrap_or(30).to_string()),
    ];

    if let Some(o) = offset {
        query_params.push(("offset".to_string(), o.to_string()));
    }

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/servicesNS/{}/{}/storage/collections", owner, app),
        "GET",
        metrics,
    )
    .await?;

    let resp: CollectionListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Create a new KVStore collection.
pub async fn create_collection(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &CreateCollectionParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<KvStoreCollection> {
    let app = params.app.as_deref().unwrap_or("search");
    let owner = params.owner.as_deref().unwrap_or("nobody");
    let url = format!(
        "{}/servicesNS/{}/{}/storage/collections",
        base_url, owner, app
    );

    let mut form_params: Vec<(String, String)> = vec![
        ("name".to_string(), params.name.clone()),
        ("output_mode".to_string(), "json".to_string()),
    ];

    if let Some(ref fields) = params.fields {
        form_params.push(("fields".to_string(), fields.clone()));
    }
    if let Some(ref accelerated_fields) = params.accelerated_fields {
        form_params.push(("acceleratedFields".to_string(), accelerated_fields.clone()));
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/servicesNS/{}/{}/storage/collections", owner, app),
        "POST",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry in create collection response".to_string())
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(&params.name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse(
            "Missing entry content in create collection response".to_string(),
        )
    })?;

    let collection: KvStoreCollection = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse collection: {}", e)))?;

    Ok(attach_entry_name(entry_name, collection))
}

/// Modify an existing KVStore collection.
#[allow(clippy::too_many_arguments)]
pub async fn modify_collection(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    collection_name: &str,
    app: &str,
    owner: &str,
    params: &ModifyCollectionParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<KvStoreCollection> {
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!(
            "/servicesNS/{}/{}/storage/collections/{}",
            owner, app, collection_name
        ))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid collection name: {}", e)))?;

    let mut form_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    if let Some(ref fields) = params.fields {
        form_params.push(("fields".to_string(), fields.clone()));
    }
    if let Some(ref accelerated_fields) = params.accelerated_fields {
        form_params.push(("acceleratedFields".to_string(), accelerated_fields.clone()));
    }
    if let Some(disabled) = params.disabled {
        form_params.push(("disabled".to_string(), disabled.to_string()));
    }

    let builder = client
        .post(url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!(
            "/servicesNS/{}/{}/storage/collections/{}",
            owner, app, collection_name
        ),
        "POST",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry in modify collection response for '{}'",
            collection_name
        ))
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(collection_name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry content in modify collection response for '{}'",
            collection_name
        ))
    })?;

    let collection: KvStoreCollection = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse collection: {}", e)))?;

    Ok(attach_entry_name(entry_name, collection))
}

/// Delete a KVStore collection.
#[allow(clippy::too_many_arguments)]
pub async fn delete_collection(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    collection_name: &str,
    app: &str,
    owner: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!(
            "/servicesNS/{}/{}/storage/collections/{}",
            owner, app, collection_name
        ))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid collection name: {}", e)))?;

    let builder = client
        .delete(url)
        .header("Authorization", format!("Bearer {}", auth_token));

    let _response = send_request_with_retry(
        builder,
        max_retries,
        &format!(
            "/servicesNS/{}/{}/storage/collections/{}",
            owner, app, collection_name
        ),
        "DELETE",
        metrics,
    )
    .await?;

    Ok(())
}

/// List records in a collection.
#[allow(clippy::too_many_arguments)]
pub async fn list_collection_records(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    collection_name: &str,
    app: &str,
    owner: &str,
    query: Option<&str>,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<KvStoreRecord>> {
    let url = format!(
        "{}/servicesNS/{}/{}/storage/collections/{}/data",
        base_url, owner, app, collection_name
    );

    let mut query_params: Vec<(String, String)> = vec![
        ("output_mode".to_string(), "json".to_string()),
        ("count".to_string(), count.unwrap_or(100).to_string()),
    ];

    if let Some(o) = offset {
        query_params.push(("offset".to_string(), o.to_string()));
    }
    if let Some(q) = query {
        query_params.push(("query".to_string(), q.to_string()));
    }

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!(
            "/servicesNS/{}/{}/storage/collections/{}/data",
            owner, app, collection_name
        ),
        "GET",
        metrics,
    )
    .await?;

    let records: Vec<KvStoreRecord> = response.json().await?;
    Ok(records)
}

/// Insert a record into a collection.
#[allow(clippy::too_many_arguments)]
pub async fn insert_collection_record(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    collection_name: &str,
    app: &str,
    owner: &str,
    record: &serde_json::Value,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<KvStoreRecord> {
    let url = format!(
        "{}/servicesNS/{}/{}/storage/collections/{}/data",
        base_url, owner, app, collection_name
    );

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .header("Content-Type", "application/json")
        .json(record);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!(
            "/servicesNS/{}/{}/storage/collections/{}/data",
            owner, app, collection_name
        ),
        "POST",
        metrics,
    )
    .await?;

    let record: KvStoreRecord = response.json().await?;
    Ok(record)
}

/// Delete a record from a collection.
#[allow(clippy::too_many_arguments)]
pub async fn delete_collection_record(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    collection_name: &str,
    app: &str,
    owner: &str,
    record_key: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    let url = format!(
        "{}/servicesNS/{}/{}/storage/collections/{}/data/{}",
        base_url, owner, app, collection_name, record_key
    );

    let builder = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", auth_token));

    let _response = send_request_with_retry(
        builder,
        max_retries,
        &format!(
            "/servicesNS/{}/{}/storage/collections/{}/data/{}",
            owner, app, collection_name, record_key
        ),
        "DELETE",
        metrics,
    )
    .await?;

    Ok(())
}
