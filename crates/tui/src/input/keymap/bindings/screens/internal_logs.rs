//! Keybindings for the Internal Logs screen.
//!
//! Responsibilities:
//! - Define bindings for internal log viewing (refresh, export, auto-refresh toggle, copy, navigate).
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
    use CurrentScreen::InternalLogs;

    vec![
        Keybinding {
            section: Section::InternalLogs,
            keys: "r",
            description: "Refresh logs",
            scope: BindingScope::Screen(InternalLogs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadMoreInternalLogs),
            handles_input: true,
        },
        Keybinding {
            section: Section::InternalLogs,
            keys: "Ctrl+e",
            description: "Export logs",
            scope: BindingScope::Screen(InternalLogs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::InternalLogs,
            keys: "a",
            description: "Toggle auto-refresh",
            scope: BindingScope::Screen(InternalLogs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::InternalLogs,
            keys: "Ctrl+c",
            description: "Copy selected log message",
            scope: BindingScope::Screen(InternalLogs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::InternalLogs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(InternalLogs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::InternalLogs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(InternalLogs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::InternalLogs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(InternalLogs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::InternalLogs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(InternalLogs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
