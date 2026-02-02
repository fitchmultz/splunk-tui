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
            section: Section::SavedSearches,
            keys: "r",
            description: "Refresh macros",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadMacros),
            handles_input: false,
        },
        Keybinding {
            section: Section::SavedSearches,
            keys: "e",
            description: "Edit macro",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::EditMacro),
            handles_input: false,
        },
        Keybinding {
            section: Section::SavedSearches,
            keys: "n",
            description: "New macro",
            scope: BindingScope::Screen(CurrentScreen::Macros),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::OpenCreateMacroDialog),
            handles_input: false,
        },
        // Note: DeleteMacro and CopyToClipboard are handled in app/input/macros.rs
        // because they require access to the currently selected macro
    ]
}
