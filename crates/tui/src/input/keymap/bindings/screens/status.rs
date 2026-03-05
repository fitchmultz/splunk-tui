//! Keybindings for status/info screens (Health, License, KVStore, Overview).
//!
//! Responsibilities:
//! - Define bindings for status screens that share a similar pattern (refresh, export, copy).
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
    use CurrentScreen::*;

    vec![
        // Health
        Keybinding {
            section: Section::Health,
            keys: "r",
            description: "Refresh health status",
            scope: BindingScope::Screen(Health),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadHealth),
            handles_input: true,
        },
        Keybinding {
            section: Section::Health,
            keys: "Ctrl+e",
            description: "Export health info",
            scope: BindingScope::Screen(Health),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Health,
            keys: "Ctrl+c",
            description: "Copy health status",
            scope: BindingScope::Screen(Health),
            matcher: None,
            action: None,
            handles_input: false,
        },
        // License
        Keybinding {
            section: Section::License,
            keys: "r",
            description: "Refresh license info",
            scope: BindingScope::Screen(License),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadLicense),
            handles_input: true,
        },
        Keybinding {
            section: Section::License,
            keys: "Ctrl+e",
            description: "Export license info",
            scope: BindingScope::Screen(License),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::License,
            keys: "Ctrl+c",
            description: "Copy license summary",
            scope: BindingScope::Screen(License),
            matcher: None,
            action: None,
            handles_input: false,
        },
        // KVStore
        Keybinding {
            section: Section::Kvstore,
            keys: "r",
            description: "Refresh KVStore status",
            scope: BindingScope::Screen(Kvstore),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadKvstore),
            handles_input: true,
        },
        Keybinding {
            section: Section::Kvstore,
            keys: "Ctrl+e",
            description: "Export KVStore status",
            scope: BindingScope::Screen(Kvstore),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Kvstore,
            keys: "Ctrl+c",
            description: "Copy KVStore status",
            scope: BindingScope::Screen(Kvstore),
            matcher: None,
            action: None,
            handles_input: false,
        },
        // Overview
        Keybinding {
            section: Section::Overview,
            keys: "r",
            description: "Refresh overview",
            scope: BindingScope::Screen(Overview),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadOverview),
            handles_input: true,
        },
        Keybinding {
            section: Section::Overview,
            keys: "Ctrl+e",
            description: "Export overview",
            scope: BindingScope::Screen(Overview),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Overview,
            keys: "Ctrl+c",
            description: "Copy overview summary",
            scope: BindingScope::Screen(Overview),
            matcher: None,
            action: None,
            handles_input: false,
        },
    ]
}
