//! Centralized keybinding catalog and input resolver.
//!
//! Responsibilities:
//! - Define a single source of truth for keybindings and their descriptions.
//! - Resolve KeyEvents into Actions without mutating App state.
//!
//! Non-responsibilities:
//! - Performing App state mutations or side effects.
//! - Handling text entry modes (those remain in App screen handlers).
//!
//! Invariants:
//! - Bindings are deterministic and stable for help/docs rendering.
//! - Resolver never mutates App state and returns at most one Action.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::CurrentScreen;

mod bindings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Global,
    Search,
    Jobs,
    JobDetails,
    Indexes,
    Cluster,
    Health,
    SavedSearches,
    InternalLogs,
    Apps,
    Users,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindingScope {
    Global,
    Screen(CurrentScreen),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Matcher {
    Key {
        code: KeyCode,
        modifiers: KeyModifiers,
    },
}

#[derive(Clone)]
pub(crate) struct Keybinding {
    pub(crate) section: Section,
    pub(crate) keys: &'static str,
    pub(crate) description: &'static str,
    pub(crate) scope: BindingScope,
    pub(crate) matcher: Option<Matcher>,
    pub(crate) action: Option<Action>,
    pub(crate) handles_input: bool,
}

impl Keybinding {
    fn matches(&self, key: KeyEvent, screen: CurrentScreen) -> bool {
        if !self.scope_applies(screen) {
            return false;
        }
        let Some(matcher) = self.matcher else {
            return false;
        };
        match matcher {
            Matcher::Key { code, modifiers } => key.code == code && key.modifiers == modifiers,
        }
    }

    fn scope_applies(&self, screen: CurrentScreen) -> bool {
        match self.scope {
            BindingScope::Global => true,
            BindingScope::Screen(s) => s == screen,
        }
    }
}

pub(crate) fn keybindings() -> Vec<Keybinding> {
    bindings::all()
}

pub(crate) fn resolve_action(screen: CurrentScreen, key: KeyEvent) -> Option<Action> {
    for binding in keybindings() {
        if !binding.handles_input {
            continue;
        }
        if binding.matches(key, screen) {
            return binding.action;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn resolves_global_quit() {
        let action = resolve_action(CurrentScreen::Search, key(KeyCode::Char('q')));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn resolves_screen_navigation_for_lists() {
        let action = resolve_action(CurrentScreen::Jobs, key(KeyCode::Char('j')));
        assert!(matches!(action, Some(Action::NavigateDown)));
    }

    #[test]
    fn ignores_list_navigation_on_search_screen() {
        let action = resolve_action(CurrentScreen::Search, key(KeyCode::Char('j')));
        assert!(action.is_none());
    }
}
