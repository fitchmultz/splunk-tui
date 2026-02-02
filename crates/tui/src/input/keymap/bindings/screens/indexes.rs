//! Keybindings for the Indexes screen.
//!
//! Responsibilities:
//! - Define bindings for index management (refresh, view, export, create, modify, delete).
//!
//! Non-responsibilities:
//! - Resolving input events or mutating App state.
//!
//! Invariants:
//! - Ordering matches the rendered help/docs expectations.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;
use crate::app::CurrentScreen;
use crate::input::keymap::{BindingScope, Keybinding, Matcher, Section};

pub(super) fn bindings() -> Vec<Keybinding> {
    use CurrentScreen::Indexes;

    vec![
        Keybinding {
            section: Section::Indexes,
            keys: "r",
            description: "Refresh indexes",
            scope: BindingScope::Screen(Indexes),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadMoreIndexes),
            handles_input: true,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "Enter",
            description: "View index details",
            scope: BindingScope::Screen(Indexes),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "Ctrl+e",
            description: "Export indexes",
            scope: BindingScope::Screen(Indexes),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "Ctrl+c",
            description: "Copy selected index name",
            scope: BindingScope::Screen(Indexes),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Indexes),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Indexes),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Indexes),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Indexes),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "c",
            description: "Create new index",
            scope: BindingScope::Screen(Indexes),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::OpenCreateIndexDialog),
            handles_input: true,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "m",
            description: "Modify selected index",
            scope: BindingScope::Screen(Indexes),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('m'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled by input handler
            handles_input: false,
        },
        Keybinding {
            section: Section::Indexes,
            keys: "d",
            description: "Delete selected index",
            scope: BindingScope::Screen(Indexes),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled by input handler
            handles_input: false,
        },
    ]
}
