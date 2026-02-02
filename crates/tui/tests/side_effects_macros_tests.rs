//! Search macro side effect handler tests.
//!
//! This module tests the macro CRUD operations and their refresh behavior.
//! Key tests verify that LoadMacros is dispatched after successful:
//! - CreateMacro
//! - UpdateMacro
//! - DeleteMacro
//!
//! This ensures the macro list stays synchronized with the server state.

mod common;

use common::*;
use wiremock::matchers::{method, path};

/// Test that creating a macro successfully dispatches LoadMacros to refresh the list.
#[tokio::test]
async fn test_create_macro_success_dispatches_refresh() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the create macro endpoint - POST to /services/admin/macros
    let create_response = serde_json::json!({
        "entry": [{
            "name": "new_test_macro",
            "content": {
                "definition": "index=main | head 10",
                "args": None::<String>,
                "description": Some("Test macro"),
                "disabled": false,
                "iseval": false,
                "validation": None::<String>,
                "errormsg": None::<String>
            }
        }]
    });
    Mock::given(method("POST"))
        .and(path("/services/admin/macros"))
        .respond_with(ResponseTemplate::new(201).set_body_json(&create_response))
        .mount(&harness.mock_server)
        .await;

    // Mock the list macros endpoint for the refresh
    let list_fixture = load_fixture("macros/list_macros.json");
    Mock::given(method("GET"))
        .and(path("/services/admin/macros"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&list_fixture))
        .mount(&harness.mock_server)
        .await;

    // Handle the CreateMacro action
    let actions = harness
        .handle_and_collect(
            Action::CreateMacro {
                name: "new_test_macro".to_string(),
                definition: "index=main | head 10".to_string(),
                args: None,
                description: Some("Test macro".to_string()),
                disabled: false,
                iseval: false,
            },
            3,
        )
        .await;

    // Verify MacroCreated(Ok) was sent
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MacroCreated(Ok(())))),
        "Should send MacroCreated(Ok)"
    );

    // Verify LoadMacros was dispatched for refresh
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadMacros)),
        "Should send LoadMacros to refresh the list after successful creation"
    );
}

/// Test that macro creation failure does NOT dispatch LoadMacros.
#[tokio::test]
async fn test_create_macro_error_does_not_dispatch_refresh() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the create macro endpoint with an error
    Mock::given(method("POST"))
        .and(path("/services/admin/macros"))
        .respond_with(ResponseTemplate::new(400).set_body_string("Invalid macro definition"))
        .mount(&harness.mock_server)
        .await;

    // Handle the CreateMacro action
    let actions = harness
        .handle_and_collect(
            Action::CreateMacro {
                name: "bad_macro".to_string(),
                definition: "".to_string(), // Empty definition should cause error
                args: None,
                description: None,
                disabled: false,
                iseval: false,
            },
            3,
        )
        .await;

    // Verify MacroCreated(Err) was sent
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MacroCreated(Err(_)))),
        "Should send MacroCreated(Err)"
    );

    // Verify LoadMacros was NOT dispatched
    assert!(
        !actions.iter().any(|a| matches!(a, Action::LoadMacros)),
        "Should NOT send LoadMacros after failed creation"
    );
}

/// Test that updating a macro successfully dispatches LoadMacros to refresh the list.
#[tokio::test]
async fn test_update_macro_success_dispatches_refresh() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the update macro endpoint - POST to specific macro path
    let update_response = serde_json::json!({
        "entry": [{
            "name": "existing_macro",
            "content": {
                "definition": "index=internal | head 20",
                "args": None::<String>,
                "description": Some("Updated description"),
                "disabled": false,
                "iseval": false,
                "validation": None::<String>,
                "errormsg": None::<String>
            }
        }]
    });
    Mock::given(method("POST"))
        .and(path("/services/admin/macros/existing_macro"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&update_response))
        .mount(&harness.mock_server)
        .await;

    // Mock the list macros endpoint for the refresh
    let list_fixture = load_fixture("macros/list_macros.json");
    Mock::given(method("GET"))
        .and(path("/services/admin/macros"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&list_fixture))
        .mount(&harness.mock_server)
        .await;

    // Handle the UpdateMacro action
    let actions = harness
        .handle_and_collect(
            Action::UpdateMacro {
                name: "existing_macro".to_string(),
                definition: Some("index=internal | head 20".to_string()),
                args: None,
                description: Some("Updated description".to_string()),
                disabled: None,
                iseval: None,
            },
            3,
        )
        .await;

    // Verify MacroUpdated(Ok) was sent
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MacroUpdated(Ok(())))),
        "Should send MacroUpdated(Ok)"
    );

    // Verify LoadMacros was dispatched for refresh
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadMacros)),
        "Should send LoadMacros to refresh the list after successful update"
    );
}

/// Test that macro update failure does NOT dispatch LoadMacros.
#[tokio::test]
async fn test_update_macro_error_does_not_dispatch_refresh() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the update macro endpoint with a 404 error (macro not found)
    Mock::given(method("POST"))
        .and(path("/services/admin/macros/nonexistent_macro"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Macro not found"))
        .mount(&harness.mock_server)
        .await;

    // Handle the UpdateMacro action
    let actions = harness
        .handle_and_collect(
            Action::UpdateMacro {
                name: "nonexistent_macro".to_string(),
                definition: Some("index=main".to_string()),
                args: None,
                description: None,
                disabled: None,
                iseval: None,
            },
            3,
        )
        .await;

    // Verify MacroUpdated(Err) was sent
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MacroUpdated(Err(_)))),
        "Should send MacroUpdated(Err)"
    );

    // Verify LoadMacros was NOT dispatched
    assert!(
        !actions.iter().any(|a| matches!(a, Action::LoadMacros)),
        "Should NOT send LoadMacros after failed update"
    );
}

/// Test that deleting a macro successfully dispatches LoadMacros to refresh the list.
#[tokio::test]
async fn test_delete_macro_success_dispatches_refresh() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the delete macro endpoint - DELETE to specific macro path
    Mock::given(method("DELETE"))
        .and(path("/services/admin/macros/macro_to_delete"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&harness.mock_server)
        .await;

    // Mock the list macros endpoint for the refresh
    let list_fixture = load_fixture("macros/list_macros.json");
    Mock::given(method("GET"))
        .and(path("/services/admin/macros"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&list_fixture))
        .mount(&harness.mock_server)
        .await;

    // Handle the DeleteMacro action
    let actions = harness
        .handle_and_collect(
            Action::DeleteMacro {
                name: "macro_to_delete".to_string(),
            },
            3,
        )
        .await;

    // Verify MacroDeleted(Ok) was sent with the macro name
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MacroDeleted(Ok(name)) if name == "macro_to_delete")),
        "Should send MacroDeleted(Ok) with macro name"
    );

    // Verify LoadMacros was dispatched for refresh
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadMacros)),
        "Should send LoadMacros to refresh the list after successful deletion"
    );
}

/// Test that macro delete failure does NOT dispatch LoadMacros.
#[tokio::test]
async fn test_delete_macro_error_does_not_dispatch_refresh() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the delete macro endpoint with a 404 error (macro not found)
    Mock::given(method("DELETE"))
        .and(path("/services/admin/macros/nonexistent_macro"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Macro not found"))
        .mount(&harness.mock_server)
        .await;

    // Handle the DeleteMacro action
    let actions = harness
        .handle_and_collect(
            Action::DeleteMacro {
                name: "nonexistent_macro".to_string(),
            },
            3,
        )
        .await;

    // Verify MacroDeleted(Err) was sent
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MacroDeleted(Err(_)))),
        "Should send MacroDeleted(Err)"
    );

    // Verify LoadMacros was NOT dispatched
    assert!(
        !actions.iter().any(|a| matches!(a, Action::LoadMacros)),
        "Should NOT send LoadMacros after failed deletion"
    );
}

/// Test that LoadMacros loads the macro list correctly.
#[tokio::test]
async fn test_load_macros_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the list macros endpoint
    let fixture = load_fixture("macros/list_macros.json");
    Mock::given(method("GET"))
        .and(path("/services/admin/macros"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Handle the LoadMacros action
    let actions = harness.handle_and_collect(Action::LoadMacros, 2).await;

    // Verify MacrosLoaded(Ok) was sent
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MacrosLoaded(Ok(_)))),
        "Should send MacrosLoaded(Ok)"
    );

    // Verify the loaded macros
    let macros_loaded = actions
        .iter()
        .find_map(|a| match a {
            Action::MacrosLoaded(Ok(macros)) => Some(macros),
            _ => None,
        })
        .expect("Should have MacrosLoaded action");

    assert!(
        !macros_loaded.is_empty(),
        "Should have loaded at least one macro"
    );
    assert_eq!(macros_loaded[0].name, "test_macro");
}

/// Test that LoadMacros action triggers the correct endpoint.
///
/// This test verifies the integration between the action and the API call,
/// ensuring that LoadMacros actually fetches from /services/admin/macros.
#[tokio::test]
async fn test_load_macros_action_integration() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the list macros endpoint
    let fixture = load_fixture("macros/list_macros.json");
    Mock::given(method("GET"))
        .and(path("/services/admin/macros"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Handle LoadMacros action
    let actions = harness.handle_and_collect(Action::LoadMacros, 2).await;

    // Verify MacrosLoaded(Ok) was sent
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MacrosLoaded(Ok(_)))),
        "LoadMacros should trigger MacrosLoaded(Ok)"
    );

    // Verify the parsed data
    let macros = actions
        .iter()
        .find_map(|a| match a {
            Action::MacrosLoaded(Ok(m)) => Some(m),
            _ => None,
        })
        .expect("Should have macros data");

    assert!(!macros.is_empty(), "Should have at least one macro");
    assert_eq!(macros[0].name, "test_macro");
}
