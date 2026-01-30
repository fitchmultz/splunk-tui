//! Config-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for config operations.
//! - Fetch config files, stanzas, and stanza details.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::runtime::side_effects::SharedClient;
use crate::ui::ToastLevel;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

/// Handle loading config files.
pub async fn handle_load_config_files(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_config_files().await {
            Ok(files) => {
                let _ = tx.send(Action::ConfigFilesLoaded(Ok(files))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::ConfigFilesLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading config stanzas for a specific config file.
pub async fn handle_load_config_stanzas(
    client: SharedClient,
    tx: Sender<Action>,
    config_file: String,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c
            .list_config_stanzas(&config_file, Some(count), Some(offset))
            .await
        {
            Ok(stanzas) => {
                let _ = tx.send(Action::ConfigStanzasLoaded(Ok(stanzas))).await;
            }
            Err(e) => {
                let error_msg = format!("Failed to load stanzas for '{}': {}", config_file, e);
                let _ = tx.send(Action::ConfigStanzasLoaded(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Notify(ToastLevel::Error, error_msg)).await;
            }
        }
    });
}
