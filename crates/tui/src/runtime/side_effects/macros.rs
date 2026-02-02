//! Search macros side effects (async API calls).
//!
//! Responsibilities:
//! - Handle async API calls for macro operations.
//! - Send results back via action channel.
//!
//! Non-responsibilities:
//! - Does not handle UI rendering (see ui/screens/macros.rs).
//! - Does not handle input (see app/input/macros.rs).

use crate::action::Action;
use crate::runtime::side_effects::SharedClient;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

/// Load macros list from the Splunk API.
pub async fn handle_load_macros(client: SharedClient, action_tx: Sender<Action>) {
    let result = {
        let mut guard = client.lock().await;
        guard.list_macros().await
    };

    let action = match result {
        Ok(macros) => Action::MacrosLoaded(Ok(macros)),
        Err(e) => Action::MacrosLoaded(Err(Arc::new(e))),
    };

    let _ = action_tx.send(action).await;
}

#[allow(clippy::too_many_arguments)]
/// Create a new macro.
pub async fn handle_create_macro(
    client: SharedClient,
    action_tx: Sender<Action>,
    name: String,
    definition: String,
    args: Option<String>,
    description: Option<String>,
    disabled: bool,
    iseval: bool,
) {
    let result = {
        let mut guard = client.lock().await;
        guard
            .create_macro(
                &name,
                &definition,
                args.as_deref(),
                description.as_deref(),
                disabled,
                iseval,
                None, // validation
                None, // errormsg
            )
            .await
    };

    let action = match result {
        Ok(()) => Action::MacroCreated(Ok(())),
        Err(e) => Action::MacroCreated(Err(Arc::new(e))),
    };

    let _ = action_tx.send(action).await;
}

#[allow(clippy::too_many_arguments)]
/// Update an existing macro.
pub async fn handle_update_macro(
    client: SharedClient,
    action_tx: Sender<Action>,
    name: String,
    definition: Option<String>,
    args: Option<String>,
    description: Option<String>,
    disabled: Option<bool>,
    iseval: Option<bool>,
) {
    let result = {
        let mut guard = client.lock().await;
        guard
            .update_macro(
                &name,
                definition.as_deref(),
                args.as_deref(),
                description.as_deref(),
                disabled,
                iseval,
                None, // validation
                None, // errormsg
            )
            .await
    };

    let action = match result {
        Ok(()) => Action::MacroUpdated(Ok(())),
        Err(e) => Action::MacroUpdated(Err(Arc::new(e))),
    };

    let _ = action_tx.send(action).await;
}

/// Delete a macro.
pub async fn handle_delete_macro(client: SharedClient, action_tx: Sender<Action>, name: String) {
    let result = {
        let mut guard = client.lock().await;
        guard.delete_macro(&name).await
    };

    let action = match result {
        Ok(()) => Action::MacroDeleted(Ok(name)),
        Err(e) => Action::MacroDeleted(Err(Arc::new(e))),
    };

    let _ = action_tx.send(action).await;
}

#[allow(dead_code)]
/// Get a single macro by name.
pub async fn handle_get_macro(client: SharedClient, action_tx: Sender<Action>, name: String) {
    let result = {
        let mut guard = client.lock().await;
        guard.get_macro(&name).await
    };

    // This is typically used for pre-populating edit dialogs
    // For now, we just log errors - the macro list will be refreshed
    if let Err(e) = result {
        let _ = action_tx
            .send(Action::Notify(
                crate::ui::ToastLevel::Error,
                format!("Failed to load macro: {}", e),
            ))
            .await;
    }
}
