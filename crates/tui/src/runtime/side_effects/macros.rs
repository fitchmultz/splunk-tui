//! Search macros side effects (async API calls).
//!
//! Responsibilities:
//! - Handle async API calls for macro operations.
//! - Send results back via action channel.
//!
//! Does NOT handle:
//! - Does not handle UI rendering (see ui/screens/macros.rs).
//! - Does not handle input (see app/input/macros.rs).

use crate::action::Action;
use crate::runtime::side_effects::{SharedClient, TaskTracker};
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
pub async fn handle_load_macros(
    client: SharedClient,
    action_tx: Sender<Action>,
    task_tracker: TaskTracker,
) {
    let _ = action_tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.list_macros().await {
            Ok(macros) => {
                let _ = action_tx.send(Action::MacrosLoaded(Ok(macros))).await;
            }
            Err(e) => {
                let _ = action_tx.send(Action::MacrosLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Create a new macro.
pub async fn handle_create_macro(
    client: SharedClient,
    action_tx: Sender<Action>,
    task_tracker: TaskTracker,
    params: CreateMacroEffectParams,
) {
    let _ = action_tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        let macro_params = MacroCreateParams {
            name: params.name,
            definition: params.definition,
            args: params.args,
            description: params.description,
            disabled: params.disabled,
            iseval: params.iseval,
            validation: None,
            errormsg: None,
        };

        match client.create_macro(macro_params).await {
            Ok(()) => {
                let _ = action_tx.send(Action::MacroCreated(Ok(()))).await;
                // Refresh macros list on success
                let _ = action_tx.send(Action::LoadMacros).await;
            }
            Err(e) => {
                let _ = action_tx.send(Action::MacroCreated(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Update an existing macro.
pub async fn handle_update_macro(
    client: SharedClient,
    action_tx: Sender<Action>,
    task_tracker: TaskTracker,
    params: UpdateMacroEffectParams,
) {
    let _ = action_tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        let macro_params = MacroUpdateParams {
            definition: params.definition,
            args: params.args,
            description: params.description,
            disabled: params.disabled,
            iseval: params.iseval,
            validation: None,
            errormsg: None,
        };

        match client.update_macro(&params.name, macro_params).await {
            Ok(()) => {
                let _ = action_tx.send(Action::MacroUpdated(Ok(()))).await;
                // Refresh macros list on success
                let _ = action_tx.send(Action::LoadMacros).await;
            }
            Err(e) => {
                let _ = action_tx.send(Action::MacroUpdated(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Delete a macro.
pub async fn handle_delete_macro(
    client: SharedClient,
    action_tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
) {
    let _ = action_tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.delete_macro(&name).await {
            Ok(()) => {
                let _ = action_tx.send(Action::MacroDeleted(Ok(name))).await;
                // Refresh macros list on success
                let _ = action_tx.send(Action::LoadMacros).await;
            }
            Err(e) => {
                let _ = action_tx.send(Action::MacroDeleted(Err(Arc::new(e)))).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use splunk_client::ClientError;

    /// Unit tests for macro action variants and params.
    ///
    /// Note: The actual refresh behavior (LoadMacros dispatch after create/update/delete)
    /// is comprehensively tested via integration tests in side_effects_macros_tests.rs.
    /// These unit tests verify the data structures and action variants.
    ///
    /// Verify that MacroCreated action exists and has the expected structure.
    #[test]
    fn test_macro_created_action_variants() {
        // Test success variant
        let success = Action::MacroCreated(Ok(()));
        assert!(matches!(success, Action::MacroCreated(Ok(()))));

        // Test error variant
        let error = ClientError::ConnectionRefused("test".to_string());
        let failure = Action::MacroCreated(Err(Arc::new(error)));
        assert!(matches!(failure, Action::MacroCreated(Err(_))));
    }

    /// Verify that MacroUpdated action exists and has the expected structure.
    #[test]
    fn test_macro_updated_action_variants() {
        // Test success variant
        let success = Action::MacroUpdated(Ok(()));
        assert!(matches!(success, Action::MacroUpdated(Ok(()))));

        // Test error variant
        let error = ClientError::ConnectionRefused("test".to_string());
        let failure = Action::MacroUpdated(Err(Arc::new(error)));
        assert!(matches!(failure, Action::MacroUpdated(Err(_))));
    }

    /// Verify that MacroDeleted action exists and has the expected structure.
    #[test]
    fn test_macro_deleted_action_variants() {
        // Test success variant
        let success = Action::MacroDeleted(Ok("test_macro".to_string()));
        assert!(matches!(success, Action::MacroDeleted(Ok(name)) if name == "test_macro"));

        // Test error variant
        let error = ClientError::ConnectionRefused("test".to_string());
        let failure = Action::MacroDeleted(Err(Arc::new(error)));
        assert!(matches!(failure, Action::MacroDeleted(Err(_))));
    }

    /// Verify that LoadMacros action exists (used for refresh).
    #[test]
    fn test_load_macros_action_exists() {
        let action = Action::LoadMacros;
        assert!(matches!(action, Action::LoadMacros));
    }

    /// Test the CreateMacroEffectParams struct construction.
    #[test]
    fn test_create_macro_effect_params() {
        let params = CreateMacroEffectParams {
            name: "test_macro".to_string(),
            definition: "index=main | head 10".to_string(),
            args: Some("arg1,arg2".to_string()),
            description: Some("Test description".to_string()),
            disabled: false,
            iseval: false,
        };

        assert_eq!(params.name, "test_macro");
        assert_eq!(params.definition, "index=main | head 10");
        assert_eq!(params.args, Some("arg1,arg2".to_string()));
        assert_eq!(params.description, Some("Test description".to_string()));
        assert!(!params.disabled);
        assert!(!params.iseval);
    }

    /// Test the UpdateMacroEffectParams struct construction.
    #[test]
    fn test_update_macro_effect_params() {
        let params = UpdateMacroEffectParams {
            name: "test_macro".to_string(),
            definition: Some("index=internal | head 5".to_string()),
            args: None,
            description: Some("Updated description".to_string()),
            disabled: Some(true),
            iseval: Some(false),
        };

        assert_eq!(params.name, "test_macro");
        assert_eq!(
            params.definition,
            Some("index=internal | head 5".to_string())
        );
        assert_eq!(params.args, None);
        assert_eq!(params.description, Some("Updated description".to_string()));
        assert_eq!(params.disabled, Some(true));
        assert_eq!(params.iseval, Some(false));
    }

    /// Test that Clone is properly derived for effect params.
    #[test]
    fn test_create_macro_effect_params_clone() {
        let params = CreateMacroEffectParams {
            name: "original".to_string(),
            definition: "search *".to_string(),
            args: None,
            description: None,
            disabled: true,
            iseval: false,
        };
        let cloned = params.clone();

        assert_eq!(cloned.name, "original");
        assert_eq!(cloned.definition, "search *");
        assert!(cloned.disabled);
    }

    /// Test that Clone is properly derived for update params.
    #[test]
    fn test_update_macro_effect_params_clone() {
        let params = UpdateMacroEffectParams {
            name: "original".to_string(),
            definition: Some("search * | head 1".to_string()),
            args: Some("arg1".to_string()),
            description: None,
            disabled: Some(false),
            iseval: Some(true),
        };
        let cloned = params.clone();

        assert_eq!(cloned.name, "original");
        assert_eq!(cloned.definition, Some("search * | head 1".to_string()));
        assert_eq!(cloned.args, Some("arg1".to_string()));
    }
}
