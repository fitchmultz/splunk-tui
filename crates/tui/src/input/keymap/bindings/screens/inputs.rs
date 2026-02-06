//! Keybindings for the Inputs screen.
//!
//! Responsibilities:
//! - Define bindings for input management (refresh, enable, disable, copy, navigate).
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
    use CurrentScreen::Inputs;

    vec![
        Keybinding {
            section: Section::Inputs,
            keys: "r",
            description: "Refresh inputs",
            scope: BindingScope::Screen(Inputs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadMoreInputs),
            handles_input: true,
        },
        Keybinding {
            section: Section::Inputs,
            keys: "e",
            description: "Enable input",
            scope: BindingScope::Screen(Inputs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled by input handler
            handles_input: true,
        },
        Keybinding {
            section: Section::Inputs,
            keys: "d",
            description: "Disable input",
            scope: BindingScope::Screen(Inputs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled by input handler
            handles_input: true,
        },
        Keybinding {
            section: Section::Inputs,
            keys: "Ctrl+c",
            description: "Copy selected input name",
            scope: BindingScope::Screen(Inputs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Inputs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Inputs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Inputs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Inputs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Inputs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Inputs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Inputs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Inputs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
