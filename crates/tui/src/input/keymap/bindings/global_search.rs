//! Global and search-screen keybindings.
//!
//! Responsibilities:
//! - Define global navigation bindings and search screen shortcuts.
//!
//! Non-responsibilities:
//! - Handling input resolution or application state updates.
//!
//! Invariants:
//! - Ordering matches the rendered help/docs expectations.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;
use crate::app::CurrentScreen;

use super::super::{BindingScope, Keybinding, Matcher, Section};

pub(super) fn bindings() -> Vec<Keybinding> {
    use CurrentScreen::*;

    vec![
        // Global
        Keybinding {
            section: Section::Global,
            keys: "?",
            description: "Help",
            scope: BindingScope::Global,
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::OpenHelpPopup),
            handles_input: true,
        },
        Keybinding {
            section: Section::Global,
            keys: "q",
            description: "Quit",
            scope: BindingScope::Global,
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::Quit),
            handles_input: true,
        },
        Keybinding {
            section: Section::Global,
            keys: "Ctrl+Q",
            description: "Quit (global)",
            scope: BindingScope::Global,
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
            }),
            action: Some(Action::Quit),
            handles_input: true,
        },
        Keybinding {
            section: Section::Global,
            keys: "Tab",
            description: "Next screen",
            scope: BindingScope::Global,
            matcher: Some(Matcher::Key {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NextScreen),
            handles_input: true,
        },
        Keybinding {
            section: Section::Global,
            keys: "Shift+Tab",
            description: "Previous screen",
            scope: BindingScope::Global,
            matcher: Some(Matcher::Key {
                code: KeyCode::BackTab,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::PreviousScreen),
            handles_input: true,
        },
        // Focus navigation (Ctrl+Tab to avoid conflict with screen navigation)
        Keybinding {
            section: Section::Global,
            keys: "Ctrl+Tab",
            description: "Next focus",
            scope: BindingScope::Global,
            matcher: Some(Matcher::Key {
                code: KeyCode::Tab,
                modifiers: KeyModifiers::CONTROL,
            }),
            action: Some(Action::NextFocus),
            handles_input: true,
        },
        Keybinding {
            section: Section::Global,
            keys: "Ctrl+Shift+Tab",
            description: "Previous focus",
            scope: BindingScope::Global,
            matcher: Some(Matcher::Key {
                code: KeyCode::BackTab,
                modifiers: KeyModifiers::CONTROL,
            }),
            action: Some(Action::PreviousFocus),
            handles_input: true,
        },
        Keybinding {
            section: Section::Global,
            keys: "Ctrl+c",
            description: "Copy to clipboard",
            scope: BindingScope::Global,
            matcher: None,
            action: None,
            handles_input: false,
        },
        // Error handling (conditional - only active when error is present)
        Keybinding {
            section: Section::Global,
            keys: "e",
            description: "Show error details (when an error is present)",
            scope: BindingScope::Global,
            matcher: None, // Handled in app.rs before keymap resolution
            action: None,
            handles_input: false,
        },
        // Search
        Keybinding {
            section: Section::Search,
            keys: "Enter",
            description: "Run search",
            scope: BindingScope::Screen(Search),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Search,
            keys: "Ctrl+e",
            description: "Export results",
            scope: BindingScope::Screen(Search),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Search,
            keys: "Ctrl+c",
            description: "Copy query (or current result)",
            scope: BindingScope::Screen(Search),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Search,
            keys: "Up/Down",
            description: "Navigate history (query)",
            scope: BindingScope::Screen(Search),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Search,
            keys: "Ctrl+j/k",
            description: "Scroll results (while typing)",
            scope: BindingScope::Screen(Search),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::CONTROL,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Search,
            keys: "Ctrl+j/k",
            description: "Scroll results (while typing)",
            scope: BindingScope::Screen(Search),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::CONTROL,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Search,
            keys: "PgDn",
            description: "Page down",
            scope: BindingScope::Screen(Search),
            matcher: Some(Matcher::Key {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::PageDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Search,
            keys: "PgUp",
            description: "Page up",
            scope: BindingScope::Screen(Search),
            matcher: Some(Matcher::Key {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::PageUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Search,
            keys: "Home",
            description: "Go to top",
            scope: BindingScope::Screen(Search),
            matcher: Some(Matcher::Key {
                code: KeyCode::Home,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::GoToTop),
            handles_input: true,
        },
        Keybinding {
            section: Section::Search,
            keys: "End",
            description: "Go to bottom",
            scope: BindingScope::Screen(Search),
            matcher: Some(Matcher::Key {
                code: KeyCode::End,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::GoToBottom),
            handles_input: true,
        },
        Keybinding {
            section: Section::Search,
            keys: "j,k,...",
            description: "Type search query",
            scope: BindingScope::Screen(Search),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Search,
            keys: "Ctrl+r",
            description: "Toggle real-time mode",
            scope: BindingScope::Screen(Search),
            matcher: None,
            action: None,
            handles_input: false,
        },
    ]
}
