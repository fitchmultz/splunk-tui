//! Keybindings for the Configs screen.
//!
//! Responsibilities:
//! - Define bindings for config file management (refresh, search stanzas, view details, back, navigate).
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
    use CurrentScreen::Configs;

    vec![
        Keybinding {
            section: Section::Configs,
            keys: "r",
            description: "Refresh config files",
            scope: BindingScope::Screen(Configs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadConfigFiles),
            handles_input: true,
        },
        Keybinding {
            section: Section::Configs,
            keys: "/",
            description: "Search stanzas",
            scope: BindingScope::Screen(Configs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::EnterSearchMode),
            handles_input: true,
        },
        Keybinding {
            section: Section::Configs,
            keys: "Enter",
            description: "View stanza details",
            scope: BindingScope::Screen(Configs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Configs,
            keys: "h",
            description: "Go back",
            scope: BindingScope::Screen(Configs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Configs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Configs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Configs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Configs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Configs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Configs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Configs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Configs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
