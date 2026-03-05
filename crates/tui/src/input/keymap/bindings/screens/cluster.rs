//! Keybindings for the Cluster screen.
//!
//! Responsibilities:
//! - Define bindings for cluster info and peer management (refresh, toggle view, navigate, export, copy).
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
    use CurrentScreen::Cluster;

    vec![
        Keybinding {
            section: Section::Cluster,
            keys: "r",
            description: "Refresh cluster info",
            scope: BindingScope::Screen(Cluster),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadClusterInfo),
            handles_input: true,
        },
        Keybinding {
            section: Section::Cluster,
            keys: "p",
            description: "Toggle peers view",
            scope: BindingScope::Screen(Cluster),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::ToggleClusterViewMode),
            handles_input: true,
        },
        Keybinding {
            section: Section::Cluster,
            keys: "j/k or Up/Down",
            description: "Navigate peers list",
            scope: BindingScope::Screen(Cluster),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Cluster,
            keys: "j/k or Up/Down",
            description: "Navigate peers list",
            scope: BindingScope::Screen(Cluster),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Cluster,
            keys: "j/k or Up/Down",
            description: "Navigate peers list",
            scope: BindingScope::Screen(Cluster),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Cluster,
            keys: "j/k or Up/Down",
            description: "Navigate peers list",
            scope: BindingScope::Screen(Cluster),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Cluster,
            keys: "Ctrl+e",
            description: "Export cluster info",
            scope: BindingScope::Screen(Cluster),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Cluster,
            keys: "Ctrl+c",
            description: "Copy cluster ID",
            scope: BindingScope::Screen(Cluster),
            matcher: None,
            action: None,
            handles_input: false,
        },
    ]
}
