//! Keybindings for the Data Models screen.
//!
//! Responsibilities:
//! - Define bindings for data model viewing (refresh, navigate).
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
    use CurrentScreen::DataModels;

    vec![
        Keybinding {
            section: Section::DataModels,
            keys: "r",
            description: "Refresh data models",
            scope: BindingScope::Screen(DataModels),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::RefreshDataModels),
            handles_input: true,
        },
        Keybinding {
            section: Section::DataModels,
            keys: "L",
            description: "Load more data models",
            scope: BindingScope::Screen(DataModels),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('L'),
                modifiers: KeyModifiers::SHIFT,
            }),
            action: Some(Action::LoadMoreDataModels),
            handles_input: true,
        },
        Keybinding {
            section: Section::DataModels,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(DataModels),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::DataModels,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(DataModels),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::DataModels,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(DataModels),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::DataModels,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(DataModels),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
