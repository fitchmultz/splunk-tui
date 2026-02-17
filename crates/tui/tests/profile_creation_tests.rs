//! Integration tests for in-TUI profile creation flow.
//!
//! Tests verify that users can create profiles entirely within the TUI
//! without needing CLI access.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splunk_tui::action::Action;
use splunk_tui::app::{App, ConnectionContext};
use splunk_tui::ui::popup::{Popup, PopupType, ProfileField};

fn create_test_app() -> App {
    App::new(None, ConnectionContext::default())
}

fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

fn enter_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
}

fn tab_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
}

fn esc_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
}

#[test]
fn test_settings_n_opens_create_profile_dialog() {
    let mut app = create_test_app();
    app.current_screen = splunk_tui::app::CurrentScreen::Settings;

    // Verify popup is closed initially
    assert!(app.popup.is_none());

    // Press 'n' in settings - this should be handled by keymap
    // The keymap sends OpenCreateProfileDialog action
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Verify CreateProfile popup is open
    assert!(app.popup.is_some());
    let popup = app.popup.as_ref().unwrap();
    assert!(matches!(popup.kind, PopupType::CreateProfile { .. }));
}

#[test]
fn test_create_profile_popup_field_navigation() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Initial field should be Name
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { selected_field, .. } = &popup.kind {
            assert_eq!(*selected_field, ProfileField::Name);
        }
    }

    // Tab should advance to next field
    let _ = app.handle_popup_input(tab_key());
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { selected_field, .. } = &popup.kind {
            assert_eq!(*selected_field, ProfileField::BaseUrl);
        }
    }

    // Tab again should advance to Username
    let _ = app.handle_popup_input(tab_key());
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { selected_field, .. } = &popup.kind {
            assert_eq!(*selected_field, ProfileField::Username);
        }
    }
}

#[test]
fn test_create_profile_popup_text_input() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Type profile name
    for c in "test-profile".chars() {
        let _ = app.handle_popup_input(key(c));
    }

    // Verify name was entered
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { name_input, .. } = &popup.kind {
            assert_eq!(name_input, "test-profile");
        }
    }
}

#[test]
fn test_create_profile_popup_cancel() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });
    assert!(app.popup.is_some());

    // Press Escape to cancel
    let _ = app.handle_popup_input(esc_key());

    // Popup should be closed
    assert!(app.popup.is_none());
}

#[test]
fn test_create_profile_popup_submit_requires_name() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Press Enter without entering a name
    let action = app.handle_popup_input(enter_key());

    // Should not produce SaveProfile action
    assert!(action.is_none());
    // Popup should still be open
    assert!(app.popup.is_some());
}

#[test]
fn test_create_profile_from_tutorial_flag() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: true,
    });

    // Verify from_tutorial flag is set
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { from_tutorial, .. } = &popup.kind {
            assert!(*from_tutorial);
        }
    }
}

#[test]
fn test_create_profile_skip_verify_toggle() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Navigate to SkipVerify field (Tab 5 times: Name -> BaseUrl -> Username -> Password -> ApiToken -> SkipVerify)
    for _ in 0..5 {
        let _ = app.handle_popup_input(tab_key());
    }

    // Get current state
    let initial_skip_verify = if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { skip_verify, .. } = &popup.kind {
            *skip_verify
        } else {
            false
        }
    } else {
        false
    };

    // Press Space to toggle
    let _ = app.handle_popup_input(key(' '));

    // Verify it toggled
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { skip_verify, .. } = &popup.kind {
            assert_ne!(*skip_verify, initial_skip_verify);
        }
    }
}

#[test]
fn test_auth_recovery_n_opens_create_profile() {
    use splunk_tui::error_details::AuthRecoveryKind;

    let mut app = create_test_app();
    app.popup = Some(
        Popup::builder(PopupType::AuthRecovery {
            kind: AuthRecoveryKind::ConnectionRefused,
        })
        .build(),
    );

    // Press 'n' to open create profile
    let action = app.handle_popup_input(key('n'));

    assert!(matches!(
        action,
        Some(Action::OpenCreateProfileDialog {
            from_tutorial: false
        })
    ));
    assert!(app.popup.is_none());
}

#[test]
fn test_create_profile_popup_with_valid_data() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Fill in profile name
    for c in "my-profile".chars() {
        let _ = app.handle_popup_input(key(c));
    }

    // Tab to Base URL field
    let _ = app.handle_popup_input(tab_key());

    // Fill in base URL
    for c in "https://localhost:8089".chars() {
        let _ = app.handle_popup_input(key(c));
    }

    // Tab to Username field
    let _ = app.handle_popup_input(tab_key());

    // Fill in username
    for c in "admin".chars() {
        let _ = app.handle_popup_input(key(c));
    }

    // Verify popup state
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile {
            name_input,
            base_url_input,
            username_input,
            ..
        } = &popup.kind
        {
            assert_eq!(name_input, "my-profile");
            assert_eq!(base_url_input, "https://localhost:8089");
            assert_eq!(username_input, "admin");
        }
    }

    // Submit should produce SaveProfile action (if name is provided)
    let action = app.handle_popup_input(enter_key());
    assert!(matches!(action, Some(Action::SaveProfile { name, .. }) if name == "my-profile"));
}

#[test]
fn test_create_profile_popup_use_keyring_toggle() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Navigate to UseKeyring field (Tab 8 times to reach the last field)
    for _ in 0..8 {
        let _ = app.handle_popup_input(tab_key());
    }

    // Verify we're on UseKeyring field
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { selected_field, .. } = &popup.kind {
            assert_eq!(*selected_field, ProfileField::UseKeyring);
        }
    }

    // Get current state
    let initial_use_keyring = if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { use_keyring, .. } = &popup.kind {
            *use_keyring
        } else {
            true // default value
        }
    } else {
        true
    };

    // Press Space to toggle
    let _ = app.handle_popup_input(key(' '));

    // Verify it toggled
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { use_keyring, .. } = &popup.kind {
            assert_ne!(*use_keyring, initial_use_keyring);
        }
    }
}

#[test]
fn test_create_profile_popup_timeout_field() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Navigate to Timeout field (Tab 6 times: Name -> BaseUrl -> Username -> Password -> ApiToken -> SkipVerify -> Timeout)
    for _ in 0..6 {
        let _ = app.handle_popup_input(tab_key());
    }

    // Verify we're on Timeout field
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { selected_field, .. } = &popup.kind {
            assert_eq!(*selected_field, ProfileField::Timeout);
        }
    }
}

#[test]
fn test_create_profile_popup_max_retries_field() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Navigate to MaxRetries field (Tab 7 times)
    for _ in 0..7 {
        let _ = app.handle_popup_input(tab_key());
    }

    // Verify we're on MaxRetries field
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { selected_field, .. } = &popup.kind {
            assert_eq!(*selected_field, ProfileField::MaxRetries);
        }
    }
}

#[test]
fn test_create_profile_popup_api_token_input() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Navigate to ApiToken field (Tab 4 times: Name -> BaseUrl -> Username -> Password -> ApiToken)
    for _ in 0..4 {
        let _ = app.handle_popup_input(tab_key());
    }

    // Verify we're on ApiToken field
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { selected_field, .. } = &popup.kind {
            assert_eq!(*selected_field, ProfileField::ApiToken);
        }
    }

    // Fill in API token
    for c in "eyJ0eXAiOiJKV1QiLCJhbGci...".chars() {
        let _ = app.handle_popup_input(key(c));
    }

    // Verify api_token was entered
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile {
            api_token_input, ..
        } = &popup.kind
        {
            assert_eq!(api_token_input, "eyJ0eXAiOiJKV1QiLCJhbGci...");
        }
    }
}

#[test]
fn test_create_profile_popup_up_navigation() {
    let mut app = create_test_app();
    app.update(Action::OpenCreateProfileDialog {
        from_tutorial: false,
    });

    // Navigate down to BaseUrl
    let _ = app.handle_popup_input(tab_key());
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { selected_field, .. } = &popup.kind {
            assert_eq!(*selected_field, ProfileField::BaseUrl);
        }
    }

    // Navigate up with Up arrow
    let _ = app.handle_popup_input(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    if let Some(popup) = &app.popup {
        if let PopupType::CreateProfile { selected_field, .. } = &popup.kind {
            assert_eq!(*selected_field, ProfileField::Name);
        }
    }
}
