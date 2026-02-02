//! Keybindings for the Saved Searches screen.
//!
//! Responsibilities:
//! - Define bindings for saved search management (refresh, export, copy, run, navigate).
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
    use CurrentScreen::SavedSearches;

    vec![
        Keybinding {
            section: Section::SavedSearches,
            keys: "r",
            description: "Refresh saved searches",
            scope: BindingScope::Screen(SavedSearches),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadSavedSearches),
            handles_input: true,
        },
        Keybinding {
            section: Section::SavedSearches,
            keys: "Ctrl+e",
            description: "Export saved searches",
            scope: BindingScope::Screen(SavedSearches),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::SavedSearches,
            keys: "Ctrl+c",
            description: "Copy selected saved search name",
            scope: BindingScope::Screen(SavedSearches),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::SavedSearches,
            keys: "Enter",
            description: "Run selected search",
            scope: BindingScope::Screen(SavedSearches),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::SavedSearches,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(SavedSearches),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::SavedSearches,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(SavedSearches),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::SavedSearches,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(SavedSearches),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::SavedSearches,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(SavedSearches),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
