//! Keybindings for the Settings screen.
//!
//! Responsibilities:
//! - Define bindings for settings management (cycle theme, toggle auto-refresh, sort, clear history, reload, profile management).
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
    use CurrentScreen::Settings;

    vec![
        Keybinding {
            section: Section::Settings,
            keys: "t",
            description: "Cycle theme",
            scope: BindingScope::Screen(Settings),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('t'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::CycleTheme),
            handles_input: true,
        },
        Keybinding {
            section: Section::Settings,
            keys: "a",
            description: "Toggle auto-refresh",
            scope: BindingScope::Screen(Settings),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Settings,
            keys: "s",
            description: "Cycle sort column",
            scope: BindingScope::Screen(Settings),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Settings,
            keys: "d",
            description: "Toggle sort direction",
            scope: BindingScope::Screen(Settings),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Settings,
            keys: "c",
            description: "Clear search history",
            scope: BindingScope::Screen(Settings),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Settings,
            keys: "r",
            description: "Reload settings",
            scope: BindingScope::Screen(Settings),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::SwitchToSettings),
            handles_input: true,
        },
        Keybinding {
            section: Section::Settings,
            keys: "p",
            description: "Switch profile",
            scope: BindingScope::Screen(Settings),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::OpenProfileSwitcher),
            handles_input: true,
        },
        Keybinding {
            section: Section::Settings,
            keys: "n",
            description: "Create new profile",
            scope: BindingScope::Screen(Settings),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::OpenCreateProfileDialog {
                from_tutorial: false,
            }),
            handles_input: true,
        },
        Keybinding {
            section: Section::Settings,
            keys: "e",
            description: "Edit selected profile",
            scope: BindingScope::Screen(Settings),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled by input handler
            handles_input: false,
        },
        Keybinding {
            section: Section::Settings,
            keys: "x",
            description: "Delete selected profile",
            scope: BindingScope::Screen(Settings),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled by input handler
            handles_input: false,
        },
        Keybinding {
            section: Section::Settings,
            keys: "?",
            description: "Replay tutorial",
            scope: BindingScope::Screen(Settings),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::StartTutorial { is_replay: true }),
            handles_input: true,
        },
    ]
}
