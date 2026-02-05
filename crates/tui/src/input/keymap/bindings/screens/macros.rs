//! Search macros keybindings.
//!
//! Responsibilities:
//! - Define keybindings for the macros screen.
//!
//! Non-responsibilities:
//! - Does not handle input parsing (see app/input/macros.rs).
//! - Does not execute actions (see runtime/side_effects/macros.rs).

use crate::action::Action;
use crate::app::CurrentScreen;
use crate::input::keymap::{BindingScope, Keybinding, Matcher, Section};
use crossterm::event::{KeyCode, KeyModifiers};

pub fn bindings() -> Vec<Keybinding> {
    vec![
        Keybinding {
            section: Section::Macros,
            keys: "r",
            description: "Refresh macros",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadMacros),
            handles_input: true,
        },
        Keybinding {
            section: Section::Macros,
            keys: "Ctrl+e",
            description: "Export macros",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Macros,
            keys: "Ctrl+c",
            description: "Copy definition",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }),
            action: None, // Handled directly in app/input/macros.rs (needs selected macro)
            handles_input: false,
        },
        Keybinding {
            section: Section::Macros,
            keys: "y",
            description: "Copy definition (vim-style)",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('y'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled directly in app/input/macros.rs (needs selected macro)
            handles_input: false,
        },
        Keybinding {
            section: Section::Macros,
            keys: "e",
            description: "Edit macro",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::EditMacro),
            handles_input: true,
        },
        Keybinding {
            section: Section::Macros,
            keys: "n",
            description: "New macro",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::OpenCreateMacroDialog),
            handles_input: true,
        },
        Keybinding {
            section: Section::Macros,
            keys: "d",
            description: "Delete macro",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Handled directly in app/input/macros.rs (needs selected macro)
            handles_input: false,
        },
        // Navigation keys
        Keybinding {
            section: Section::Macros,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Macros,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Macros,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Macros,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Macros,
            keys: "PgDn",
            description: "Page down",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Page down handled directly in app/input/macros.rs
            handles_input: false,
        },
        Keybinding {
            section: Section::Macros,
            keys: "PgUp",
            description: "Page up",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Page up handled directly in app/input/macros.rs
            handles_input: false,
        },
        Keybinding {
            section: Section::Macros,
            keys: "Home",
            description: "Go to top",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Home,
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // Home handled directly in app/input/macros.rs
            handles_input: false,
        },
        Keybinding {
            section: Section::Macros,
            keys: "End",
            description: "Go to bottom",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::End,
                modifiers: KeyModifiers::NONE,
            }),
            action: None, // End handled directly in app/input/macros.rs
            handles_input: false,
        },
    ]
}
