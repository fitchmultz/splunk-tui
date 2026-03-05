//! Keybindings for the Audit screen.
//!
//! Responsibilities:
//! - Define bindings for audit events screen (refresh, export, copy, navigate).
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
    use CurrentScreen::Audit;

    vec![
        Keybinding {
            section: Section::Audit,
            keys: "r",
            description: "Refresh audit events",
            scope: BindingScope::Screen(Audit),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadRecentAuditEvents { count: 100 }),
            handles_input: true,
        },
        Keybinding {
            section: Section::Audit,
            keys: "Ctrl+e",
            description: "Export audit events",
            scope: BindingScope::Screen(Audit),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Audit,
            keys: "Ctrl+c",
            description: "Copy selected event",
            scope: BindingScope::Screen(Audit),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Audit,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Audit),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Audit,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Audit),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Audit,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Audit),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Audit,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Audit),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
