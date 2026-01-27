//! Job list and job detail keybindings.
//!
//! Responsibilities:
//! - Define job list navigation and job inspect shortcuts.
//!
//! Non-responsibilities:
//! - Resolving input events or mutating App state.
//!
//! Invariants:
//! - Ordering matches the rendered help/docs expectations.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;
use crate::app::CurrentScreen;

use super::super::{BindingScope, Keybinding, Matcher, Section};

pub(super) fn bindings() -> Vec<Keybinding> {
    use CurrentScreen::*;

    vec![
        // Jobs
        Keybinding {
            section: Section::Jobs,
            keys: "r",
            description: "Refresh jobs",
            scope: BindingScope::Screen(Jobs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('r'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::LoadJobs),
            handles_input: true,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "e",
            description: "Export jobs",
            scope: BindingScope::Screen(Jobs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "Ctrl+c",
            description: "Copy selected SID",
            scope: BindingScope::Screen(Jobs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "a",
            description: "Toggle auto-refresh",
            scope: BindingScope::Screen(Jobs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "s",
            description: "Cycle sort column",
            scope: BindingScope::Screen(Jobs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('s'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::CycleSortColumn),
            handles_input: true,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "/",
            description: "Filter jobs",
            scope: BindingScope::Screen(Jobs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::EnterSearchMode),
            handles_input: true,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "Space",
            description: "Toggle job selection",
            scope: BindingScope::Screen(Jobs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "c",
            description: "Cancel selected job(s)",
            scope: BindingScope::Screen(Jobs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "d",
            description: "Delete selected job(s)",
            scope: BindingScope::Screen(Jobs),
            matcher: None,
            action: None,
            handles_input: false,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Jobs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Jobs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Jobs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateDown),
            handles_input: true,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "j/k or Up/Down",
            description: "Navigate list",
            scope: BindingScope::Screen(Jobs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::NavigateUp),
            handles_input: true,
        },
        Keybinding {
            section: Section::Jobs,
            keys: "Enter",
            description: "Inspect job",
            scope: BindingScope::Screen(Jobs),
            matcher: Some(Matcher::Key {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::InspectJob),
            handles_input: true,
        },
        // Job Details
        Keybinding {
            section: Section::JobDetails,
            keys: "Esc",
            description: "Back to jobs",
            scope: BindingScope::Screen(JobInspect),
            matcher: Some(Matcher::Key {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
            }),
            action: Some(Action::ExitInspectMode),
            handles_input: true,
        },
        Keybinding {
            section: Section::JobDetails,
            keys: "Ctrl+c",
            description: "Copy job SID",
            scope: BindingScope::Screen(JobInspect),
            matcher: None,
            action: None,
            handles_input: false,
        },
    ]
}
