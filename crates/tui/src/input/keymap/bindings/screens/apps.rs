//! Keybindings for the Apps screen.
//!
//! Responsibilities:
//! - Define bindings for app management (refresh, export, copy, navigate, enable, disable, install, remove).
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
    use CurrentScreen::Apps;

    vec![
        Keybinding {
            section: Section::Apps,
            keys: "r",
            description: "Refresh apps",
            scope: BindingScope::Screen(Apps),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadMoreApps),
            handles_input: true,
        },
        Keybinding {
            section: Section::Apps,
            keys: "Ctrl+e",
            description: "Export apps",
            scope: BindingScope::Screen(Apps),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Apps,
            keys: "Ctrl+c",
            description: "Copy selected app name",
            scope: BindingScope::Screen(Apps),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Apps,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Apps),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Apps,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Apps),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Apps,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Apps),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Apps,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Apps),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Apps,
            keys: "e",
            description: "Enable selected app",
            scope: BindingScope::Screen(Apps),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled directly in input handler
            handles_input: false,
        },
        Keybinding {
            section: Section::Apps,
            keys: "d",
            description: "Disable selected app",
            scope: BindingScope::Screen(Apps),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled directly in input handler
            handles_input: false,
        },
        Keybinding {
            section: Section::Apps,
            keys: "i",
            description: "Install app from .spl file",
            scope: BindingScope::Screen(Apps),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('i'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled directly in input handler
            handles_input: false,
        },
        Keybinding {
            section: Section::Apps,
            keys: "x",
            description: "Remove selected app",
            scope: BindingScope::Screen(Apps),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled directly in input handler
            handles_input: false,
        },
    ]
}
