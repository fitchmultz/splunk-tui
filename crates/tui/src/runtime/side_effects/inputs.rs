//! Input-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for input operations.
//! - Fetch input lists, enable inputs, disable inputs.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::runtime::side_effects::SharedClient;
use crate::ui::ToastLevel;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

/// Handle loading inputs with pagination support.
///
/// Emits `InputsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreInputsLoaded` when offset > 0 (pagination).
pub async fn handle_load_inputs(client: SharedClient, tx: Sender<Action>, count: u64, offset: u64) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_inputs(Some(count), Some(offset)).await {
            Ok(inputs) => {
                if offset == 0 {
                    let _ = tx.send(Action::InputsLoaded(Ok(inputs))).await;
                } else {
                    let _ = tx.send(Action::MoreInputsLoaded(Ok(inputs))).await;
                }
            }
            Err(e) => {
                if offset == 0 {
                    let _ = tx.send(Action::InputsLoaded(Err(Arc::new(e)))).await;
                } else {
                    let _ = tx.send(Action::MoreInputsLoaded(Err(Arc::new(e)))).await;
                }
            }
        }
    });
}

/// Handle enabling an input.
pub async fn handle_enable_input(
    client: SharedClient,
    tx: Sender<Action>,
    input_type: String,
    name: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.enable_input(&input_type, &name).await {
            Ok(_) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Input '{}' enabled successfully", name),
                    ))
                    .await;
                // Refresh inputs list (reset pagination)
                let _ = tx
                    .send(Action::LoadInputs {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to enable input '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle disabling an input.
pub async fn handle_disable_input(
    client: SharedClient,
    tx: Sender<Action>,
    input_type: String,
    name: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.disable_input(&input_type, &name).await {
            Ok(_) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Input '{}' disabled successfully", name),
                    ))
                    .await;
                // Refresh inputs list (reset pagination)
                let _ = tx
                    .send(Action::LoadInputs {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to disable input '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}
