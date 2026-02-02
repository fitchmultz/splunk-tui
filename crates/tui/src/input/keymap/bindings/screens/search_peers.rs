//! Keybindings for the Search Peers screen.
//!
//! Responsibilities:
//! - Define bindings for search peer management (refresh, export, copy, navigate).
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
    use CurrentScreen::SearchPeers;

    vec![
        Keybinding {
            section: Section::SearchPeers,
            keys: "r",
            description: "Refresh search peers",
            scope: BindingScope::Screen(SearchPeers),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadSearchPeers {
                count: 30,
                offset: 0,
            }),
            handles_input: true,
        },
        Keybinding {
            section: Section::SearchPeers,
            keys: "Ctrl+e",
            description: "Export search peers",
            scope: BindingScope::Screen(SearchPeers),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::SearchPeers,
            keys: "Ctrl+c",
            description: "Copy selected peer name",
            scope: BindingScope::Screen(SearchPeers),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::SearchPeers,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(SearchPeers),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::SearchPeers,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(SearchPeers),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::SearchPeers,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(SearchPeers),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::SearchPeers,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(SearchPeers),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
