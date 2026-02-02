//! Keybindings for monitoring screens (Multi-Instance, Fired Alerts, Forwarders, Lookups).
//!
//! Responsibilities:
//! - Define bindings for monitoring screens (refresh, export, copy, navigate).
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
    use CurrentScreen::*;

    vec![
        // Multi-Instance
        Keybinding {
            section: Section::MultiInstance,
            keys: "r",
            description: "Refresh multi-instance dashboard",
            scope: BindingScope::Screen(MultiInstance),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadMultiInstanceOverview),
            handles_input: true,
        },
        Keybinding {
            section: Section::MultiInstance,
            keys: "Ctrl+e",
            description: "Export multi-instance data",
            scope: BindingScope::Screen(MultiInstance),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::MultiInstance,
            keys: "Ctrl+c",
            description: "Copy instance summary",
            scope: BindingScope::Screen(MultiInstance),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::MultiInstance,
            keys: "j/k or Up/Down",
            description: "Navigate instances",
            scope: BindingScope::Screen(MultiInstance),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::MultiInstance,
            keys: "j/k or Up/Down",
            description: "Navigate instances",
            scope: BindingScope::Screen(MultiInstance),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::MultiInstance,
            keys: "j/k or Up/Down",
            description: "Navigate instances",
            scope: BindingScope::Screen(MultiInstance),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::MultiInstance,
            keys: "j/k or Up/Down",
            description: "Navigate instances",
            scope: BindingScope::Screen(MultiInstance),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        // Fired Alerts
        Keybinding {
            section: Section::FiredAlerts,
            keys: "r",
            description: "Refresh fired alerts",
            scope: BindingScope::Screen(FiredAlerts),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadFiredAlerts),
            handles_input: true,
        },
        Keybinding {
            section: Section::FiredAlerts,
            keys: "Ctrl+e",
            description: "Export fired alerts",
            scope: BindingScope::Screen(FiredAlerts),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::FiredAlerts,
            keys: "Ctrl+c",
            description: "Copy selected alert name",
            scope: BindingScope::Screen(FiredAlerts),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::FiredAlerts,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(FiredAlerts),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::FiredAlerts,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(FiredAlerts),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::FiredAlerts,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(FiredAlerts),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::FiredAlerts,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(FiredAlerts),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        // Forwarders
        Keybinding {
            section: Section::Forwarders,
            keys: "r",
            description: "Refresh forwarders",
            scope: BindingScope::Screen(Forwarders),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadForwarders {
                count: 30,
                offset: 0,
            }),
            handles_input: true,
        },
        Keybinding {
            section: Section::Forwarders,
            keys: "Ctrl+e",
            description: "Export forwarders",
            scope: BindingScope::Screen(Forwarders),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Forwarders,
            keys: "Ctrl+c",
            description: "Copy selected forwarder name",
            scope: BindingScope::Screen(Forwarders),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Forwarders,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Forwarders),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Forwarders,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Forwarders),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Forwarders,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Forwarders),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Forwarders,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Forwarders),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        // Lookups
        Keybinding {
            section: Section::Lookups,
            keys: "r",
            description: "Refresh lookup tables",
            scope: BindingScope::Screen(Lookups),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadLookups {
                count: 30,
                offset: 0,
            }),
            handles_input: true,
        },
        Keybinding {
            section: Section::Lookups,
            keys: "Ctrl+e",
            description: "Export lookup tables",
            scope: BindingScope::Screen(Lookups),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Lookups,
            keys: "Ctrl+c",
            description: "Copy selected lookup name",
            scope: BindingScope::Screen(Lookups),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Lookups,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Lookups),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Lookups,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Lookups),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Lookups,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Lookups),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Lookups,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Lookups),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
    ]
}
