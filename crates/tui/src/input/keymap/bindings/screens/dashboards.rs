//! Keybindings for the Dashboards screen.
//!
//! Responsibilities:
//! - Define bindings for dashboard viewing (refresh, navigate).
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
    use CurrentScreen::Dashboards;

    vec![
        Keybinding {
            section: Section::Dashboards,
            keys: "r",
            description: "Refresh dashboards",
            scope: BindingScope::Screen(Dashboards),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::RefreshDashboards),
            handles_input: true,
        },
        Keybinding {
            section: Section::Dashboards,
            keys: "L",
            description: "Load more dashboards",
            scope: BindingScope::Screen(Dashboards),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('L'),
                modifiers: KeyModifiers::SHIFT,
            }),
            action: Some(Action::LoadMoreDashboards),
            handles_input: true,
        },
        Keybinding {
            section: Section::Dashboards,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Dashboards),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Dashboards,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Dashboards),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Dashboards,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Dashboards),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Dashboards,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Dashboards),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
