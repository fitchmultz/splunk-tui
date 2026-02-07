//! Side effects for lookup table operations.
//!
//! Responsibilities:
//! - Handle LoadLookups action to fetch lookup tables
//! - Handle LoadMoreLookups action for pagination
//! - Handle DownloadLookup action to download lookup files
//! - Handle DeleteLookup action to delete lookup files
//!
//! Does NOT handle:
//! - UI rendering (handled by screen module)
//! - Input handling (handled by input handlers)

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use crate::action::Action;
use crate::runtime::side_effects::SharedClient;

/// Handle loading lookup tables with pagination support.
///
/// Emits `LookupsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreLookupsLoaded` when offset > 0 (pagination).
pub async fn handle_load_lookups(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client
            .list_lookup_tables(Some(count as u32), Some(offset as u32))
            .await
        {
            Ok(lookups) => {
                if offset == 0 {
                    let _ = tx.send(Action::LookupsLoaded(Ok(lookups))).await;
                } else {
                    let _ = tx.send(Action::MoreLookupsLoaded(Ok(lookups))).await;
                }
            }
            Err(e) => {
                let arc_err = Arc::new(e);
                if offset == 0 {
                    let _ = tx.send(Action::LookupsLoaded(Err(arc_err))).await;
                } else {
                    let _ = tx.send(Action::MoreLookupsLoaded(Err(arc_err))).await;
                }
            }
        }
        let _ = tx.send(Action::Loading(false)).await;
    });
}

/// Handle downloading a lookup table file.
///
/// Emits `LookupDownloaded` with the lookup name on success, or error on failure.
pub async fn handle_download_lookup(
    client: SharedClient,
    tx: Sender<Action>,
    name: String,
    app: Option<String>,
    owner: Option<String>,
    output_path: PathBuf,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client
            .download_lookup_table(&name, app.as_deref(), owner.as_deref())
            .await
        {
            Ok(content) => {
                // Write content to file
                match tokio::fs::write(&output_path, content).await {
                    Ok(_) => {
                        let _ = tx.send(Action::LookupDownloaded(Ok(name.clone()))).await;
                    }
                    Err(e) => {
                        let err = Arc::new(splunk_client::ClientError::InvalidResponse(format!(
                            "Failed to write file: {}",
                            e
                        )));
                        let _ = tx.send(Action::LookupDownloaded(Err(err))).await;
                    }
                }
            }
            Err(e) => {
                let arc_err = Arc::new(e);
                let _ = tx.send(Action::LookupDownloaded(Err(arc_err))).await;
            }
        }
        let _ = tx.send(Action::Loading(false)).await;
    });
}

/// Handle deleting a lookup table file.
///
/// Emits `LookupDeleted` with the lookup name on success, or error on failure.
pub async fn handle_delete_lookup(
    client: SharedClient,
    tx: Sender<Action>,
    name: String,
    app: Option<String>,
    owner: Option<String>,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client
            .delete_lookup_table(&name, app.as_deref(), owner.as_deref())
            .await
        {
            Ok(_) => {
                let _ = tx.send(Action::LookupDeleted(Ok(name))).await;
            }
            Err(e) => {
                let arc_err = Arc::new(e);
                let _ = tx.send(Action::LookupDeleted(Err(arc_err))).await;
            }
        }
        let _ = tx.send(Action::Loading(false)).await;
    });
}
