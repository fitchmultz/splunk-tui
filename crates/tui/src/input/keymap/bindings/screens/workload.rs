//! Keybindings for the Workload Management screen.
//!
//! Responsibilities:
//! - Define bindings for workload management (refresh, toggle view, navigate).
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
    use CurrentScreen::WorkloadManagement;

    vec![
        Keybinding {
            section: Section::Workload,
            keys: "r",
            description: "Refresh workload",
            scope: BindingScope::Screen(WorkloadManagement),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadWorkloadPools {
                count: 30,
                offset: 0,
            }),
            handles_input: true,
        },
        Keybinding {
            section: Section::Workload,
            keys: "w",
            description: "Toggle pools/rules",
            scope: BindingScope::Screen(WorkloadManagement),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::ToggleWorkloadViewMode),
            handles_input: true,
        },
        Keybinding {
            section: Section::Workload,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(WorkloadManagement),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Workload,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(WorkloadManagement),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Workload,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(WorkloadManagement),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Workload,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(WorkloadManagement),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Workload,
            keys: "Ctrl+e",
            description: "Export workload",
            scope: BindingScope::Screen(WorkloadManagement),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Workload,
            keys: "n",
            description: "Load more",
            scope: BindingScope::Screen(WorkloadManagement),
            matcher: None,
            action: None,
            handles_input: false,
        },
    ]
}
