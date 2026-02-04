//! Keybindings for the SHC screen.
//!
//! Responsibilities:
//! - Define bindings for SHC info and member management (refresh, toggle view, navigate, export, copy).
//!
//! Non-responsibilities:
//! - Resolving input events or mutating App state.
//!
//! Invariants:
//! - Ordering matches the rendered help/docs expectations.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;
use crate::input::keymap::{BindingScope, Keybinding, Matcher, Section};

pub(super) fn bindings() -> Vec<Keybinding> {
    use crate::app::state::CurrentScreen::Shc;

    vec![
        Keybinding {
            section: Section::Shc,
            keys: "r",
            description: "Refresh SHC info",
            scope: BindingScope::Screen(Shc),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadShcStatus),
            handles_input: true,
        },
        Keybinding {
            section: Section::Shc,
            keys: "m",
            description: "Toggle members view",
            scope: BindingScope::Screen(Shc),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('m'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::ToggleShcViewMode),
            handles_input: true,
        },
        Keybinding {
            section: Section::Shc,
            keys: "j/k or Up/Down",
            description: "Navigate members list",
            scope: BindingScope::Screen(Shc),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Shc,
            keys: "j/k or Up/Down",
            description: "Navigate members list",
            scope: BindingScope::Screen(Shc),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Shc,
            keys: "j/k or Up/Down",
            description: "Navigate members list",
            scope: BindingScope::Screen(Shc),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Shc,
            keys: "j/k or Up/Down",
            description: "Navigate members list",
            scope: BindingScope::Screen(Shc),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Shc,
            keys: "Ctrl+e",
            description: "Export SHC info",
            scope: BindingScope::Screen(Shc),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Shc,
            keys: "Ctrl+c",
            description: "Copy captain URI",
            scope: BindingScope::Screen(Shc),
            matcher: None,
            action: None,
            handles_input: false,
        },
    ]
}
