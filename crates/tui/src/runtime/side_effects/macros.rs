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
use splunk_client::{MacroCreateParams, MacroUpdateParams};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

/// Parameters for creating a macro via side effects.
#[derive(Debug, Clone)]
pub struct CreateMacroEffectParams {
    /// Name of the macro
    pub name: String,
    /// Macro definition
    pub definition: String,
    /// Optional arguments
    pub args: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Whether the macro is disabled
    pub disabled: bool,
    /// Whether the macro is an eval expression
    pub iseval: bool,
}

/// Parameters for updating a macro via side effects.
#[derive(Debug, Clone)]
pub struct UpdateMacroEffectParams {
    /// Name of the macro
    pub name: String,
    /// Optional new definition
    pub definition: Option<String>,
    /// Optional new arguments
    pub args: Option<String>,
    /// Optional new description
    pub description: Option<String>,
    /// Optional disabled state
    pub disabled: Option<bool>,
    /// Optional iseval flag
    pub iseval: Option<bool>,
}

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

/// Create a new macro.
pub async fn handle_create_macro(
    client: SharedClient,
    action_tx: Sender<Action>,
    params: CreateMacroEffectParams,
) {
    let macro_params = MacroCreateParams {
        name: &params.name,
        definition: &params.definition,
        args: params.args.as_deref(),
        description: params.description.as_deref(),
        disabled: params.disabled,
        iseval: params.iseval,
        validation: None,
        errormsg: None,
    };

    let result = {
        let mut guard = client.lock().await;
        guard.create_macro(macro_params).await
    };

    let is_ok = result.is_ok();
    let action = match result {
        Ok(()) => Action::MacroCreated(Ok(())),
        Err(e) => Action::MacroCreated(Err(Arc::new(e))),
    };

    let _ = action_tx.send(action).await;

    // Refresh macros list on success
    if is_ok {
        let _ = action_tx.send(Action::LoadMacros).await;
    }
}

/// Update an existing macro.
pub async fn handle_update_macro(
    client: SharedClient,
    action_tx: Sender<Action>,
    params: UpdateMacroEffectParams,
) {
    let macro_params = MacroUpdateParams {
        name: &params.name,
        definition: params.definition.as_deref(),
        args: params.args.as_deref(),
        description: params.description.as_deref(),
        disabled: params.disabled,
        iseval: params.iseval,
        validation: None,
        errormsg: None,
    };

    let result = {
        let mut guard = client.lock().await;
        guard.update_macro(macro_params).await
    };

    let is_ok = result.is_ok();
    let action = match result {
        Ok(()) => Action::MacroUpdated(Ok(())),
        Err(e) => Action::MacroUpdated(Err(Arc::new(e))),
    };

    let _ = action_tx.send(action).await;

    // Refresh macros list on success
    if is_ok {
        let _ = action_tx.send(Action::LoadMacros).await;
    }
}

/// Delete a macro.
pub async fn handle_delete_macro(client: SharedClient, action_tx: Sender<Action>, name: String) {
    let result = {
        let mut guard = client.lock().await;
        guard.delete_macro(&name).await
    };

    let is_ok = result.is_ok();
    let action = match result {
        Ok(()) => Action::MacroDeleted(Ok(name)),
        Err(e) => Action::MacroDeleted(Err(Arc::new(e))),
    };

    let _ = action_tx.send(action).await;

    // Refresh macros list on success
    if is_ok {
        let _ = action_tx.send(Action::LoadMacros).await;
    }
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
