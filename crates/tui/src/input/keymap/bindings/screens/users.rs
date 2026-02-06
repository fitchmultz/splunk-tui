//! Keybindings for user and role management screens (Users, Roles).
//!
//! Responsibilities:
//! - Define bindings for user and role management (refresh, create, modify, delete, export, copy, navigate).
//!
//! Does NOT handle:
//! - Resolving input events or mutating App state.
//!
//! Invariants:
//! - Ordering matches the rendered help/docs expectations.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;
use crate::app::CurrentScreen;
use crate::input::keymap::{BindingScope, Keybinding, Matcher, Section};

pub(super) fn bindings() -> Vec<Keybinding> {
    use CurrentScreen::*;

    vec![
        // Users
        Keybinding {
            section: Section::Users,
            keys: "r",
            description: "Refresh users",
            scope: BindingScope::Screen(Users),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadMoreUsers),
            handles_input: true,
        },
        Keybinding {
            section: Section::Users,
            keys: "Ctrl+e",
            description: "Export users",
            scope: BindingScope::Screen(Users),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Users,
            keys: "Ctrl+c",
            description: "Copy selected username",
            scope: BindingScope::Screen(Users),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Users,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Users),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Users,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Users),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Users,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Users),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Users,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Users),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        // Roles
        Keybinding {
            section: Section::Roles,
            keys: "r",
            description: "Refresh roles",
            scope: BindingScope::Screen(Roles),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadRoles {
                count: 100,
                offset: 0,
            }),
            handles_input: true,
        },
        Keybinding {
            section: Section::Roles,
            keys: "c",
            description: "Create new role",
            scope: BindingScope::Screen(Roles),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Roles,
            keys: "m",
            description: "Modify selected role",
            scope: BindingScope::Screen(Roles),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Roles,
            keys: "d",
            description: "Delete selected role",
            scope: BindingScope::Screen(Roles),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Roles,
            keys: "Ctrl+e",
            description: "Export roles",
            scope: BindingScope::Screen(Roles),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Roles,
            keys: "Ctrl+c",
            description: "Copy selected role name",
            scope: BindingScope::Screen(Roles),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Roles,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Roles),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Roles,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Roles),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Roles,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Roles),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Roles,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Roles),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
