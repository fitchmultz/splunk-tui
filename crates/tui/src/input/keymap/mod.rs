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
pub mod overrides;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Global,
    Search,
    Jobs,
    JobDetails,
    Indexes,
    Cluster,
    Health,
    License,
    Kvstore,
    SavedSearches,
    InternalLogs,
    Apps,
    Users,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingScope {
    Global,
    Screen(CurrentScreen),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matcher {
    Key {
        code: KeyCode,
        modifiers: KeyModifiers,
    },
}

#[derive(Clone)]
pub struct Keybinding {
    pub section: Section,
    pub keys: &'static str,
    pub description: &'static str,
    pub scope: BindingScope,
    pub matcher: Option<Matcher>,
    pub action: Option<Action>,
    pub handles_input: bool,
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

pub fn keybindings() -> Vec<Keybinding> {
    bindings::all()
}

pub fn resolve_action(screen: CurrentScreen, key: KeyEvent) -> Option<Action> {
    // First: check user overrides (if initialized)
    if let Some(action) = overrides::resolve_override(key) {
        return Some(action);
    }

    // Second: fall back to default bindings
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

/// Returns footer hints for the given screen.
///
/// Returns the top 3-4 most relevant keybindings for the screen, formatted as
/// compact "key:action" strings. Excludes navigation keys (Tab, Shift+Tab, q)
/// that are always shown in the generic footer.
///
/// Note: Search screen has two input modes (QueryFocused/ResultsFocused) but
/// both modes share the same keybinding hints since the keymap doesn't
/// distinguish between them; mode-specific behavior is handled at input time.
pub(crate) fn footer_hints(screen: CurrentScreen) -> Vec<(&'static str, &'static str)> {
    use std::collections::BTreeSet;

    let section = screen_to_section(screen);

    // Get unique entries for this section, filtering out navigation keys
    let mut seen = BTreeSet::new();
    let mut hints = Vec::new();

    // Navigation keys that are always shown in the generic footer
    let nav_keys: &[&str] = &["Tab", "Shift+Tab", "q", "Ctrl+Q", "?"];

    for binding in keybindings() {
        if binding.section != section {
            continue;
        }

        // Skip navigation keys that are always shown
        if nav_keys.contains(&binding.keys) {
            continue;
        }

        // Skip entries with duplicate descriptions (e.g., j/k navigation)
        let key = (binding.keys, binding.description);
        if !seen.insert(key) {
            continue;
        }

        // Shorten descriptions for footer display
        let short_desc = shorten_description(binding.description);
        hints.push((binding.keys, short_desc));
    }

    // Limit to top 4 hints, prioritizing by relevance
    prioritize_hints(&mut hints, screen);
    hints.truncate(4);

    hints
}

/// Map CurrentScreen to Section enum.
fn screen_to_section(screen: CurrentScreen) -> Section {
    match screen {
        CurrentScreen::Search => Section::Search,
        CurrentScreen::Jobs => Section::Jobs,
        CurrentScreen::JobInspect => Section::JobDetails,
        CurrentScreen::Indexes => Section::Indexes,
        CurrentScreen::Cluster => Section::Cluster,
        CurrentScreen::Health => Section::Health,
        CurrentScreen::License => Section::License,
        CurrentScreen::Kvstore => Section::Kvstore,
        CurrentScreen::SavedSearches => Section::SavedSearches,
        CurrentScreen::InternalLogs => Section::InternalLogs,
        CurrentScreen::Apps => Section::Apps,
        CurrentScreen::Users => Section::Users,
        CurrentScreen::Settings => Section::Settings,
    }
}

/// Shorten descriptions for compact footer display.
fn shorten_description(desc: &'static str) -> &'static str {
    match desc {
        "Refresh jobs" => "Refresh",
        "Refresh indexes" => "Refresh",
        "Refresh cluster info" => "Refresh",
        "Refresh health status" => "Refresh",
        "Refresh saved searches" => "Refresh",
        "Refresh logs" => "Refresh",
        "Refresh apps" => "Refresh",
        "Refresh users" => "Refresh",
        "Reload settings" => "Reload",
        "Run search" => "Run",
        "Export jobs" => "Export",
        "Export results" => "Export",
        "Export indexes" => "Export",
        "Export cluster info" => "Export",
        "Export health info" => "Export",
        "Export saved searches" => "Export",
        "Export logs" => "Export",
        "Export apps" => "Export",
        "Export users" => "Export",
        "Refresh KVStore status" => "Refresh",
        "Export KVStore status" => "Export",
        "Copy KVStore status" => "Copy",
        "Cycle sort column" => "Sort",
        "Toggle sort direction" => "Direction",
        "Filter jobs" => "Filter",
        "Toggle job selection" => "Select",
        "Cancel selected job(s)" => "Cancel",
        "Delete selected job(s)" => "Delete",
        "Inspect job" => "Inspect",
        "Back to jobs" => "Back",
        "View index details" => "Details",
        "Toggle peers view" => "Peers",
        "Run selected search" => "Run",
        "Toggle auto-refresh" => "Auto",
        "Enable selected app" => "Enable",
        "Disable selected app" => "Disable",
        "Clear search history" => "Clear",
        "Cycle theme" => "Theme",
        "Copy selected SID" => "Copy SID",
        "Copy job SID" => "Copy SID",
        "Copy selected index name" => "Copy",
        "Copy cluster ID" => "Copy",
        "Copy selected saved search name" => "Copy",
        "Copy selected log message" => "Copy",
        "Copy selected app name" => "Copy",
        "Copy selected username" => "Copy",
        "Copy query (or current result)" => "Copy",
        "Copy to clipboard" => "Copy",
        "Navigate list" => "Navigate",
        "Navigate peers list" => "Navigate",
        "Navigate history (query)" => "History",
        "Scroll results (while typing)" => "Scroll",
        "Page down" => "PgDn",
        "Page up" => "PgUp",
        "Go to top" => "Top",
        "Go to bottom" => "Bottom",
        "Type search query" => "Type",
        _ => desc,
    }
}

/// Prioritize hints based on screen and relevance.
fn prioritize_hints(hints: &mut Vec<(&'static str, &'static str)>, screen: CurrentScreen) {
    // Define priority order for each screen
    let priority_keys: &[&str] = match screen {
        CurrentScreen::Search => &["Enter", "Ctrl+e", "PgDn", "PgUp", "Ctrl+j/k", "Home", "End"],
        CurrentScreen::Jobs => &["r", "/", "s", "a", "Space", "c", "d", "Enter"],
        CurrentScreen::JobInspect => &["Esc", "Ctrl+c"],
        CurrentScreen::Indexes => &["r", "Enter", "j/k or Up/Down"],
        CurrentScreen::Cluster => &["r", "p", "j/k or Up/Down"],
        CurrentScreen::Health => &["r"],
        CurrentScreen::License => &["r"],
        CurrentScreen::Kvstore => &["r"],
        CurrentScreen::SavedSearches => &["r", "Enter", "j/k or Up/Down"],
        CurrentScreen::InternalLogs => &["r", "a", "j/k or Up/Down"],
        CurrentScreen::Apps => &["r", "e", "d", "j/k or Up/Down"],
        CurrentScreen::Users => &["r", "j/k or Up/Down"],
        CurrentScreen::Settings => &["t", "a", "s", "d", "c", "r"],
    };

    // Sort hints by priority order
    hints.sort_by(|(key_a, _), (key_b, _)| {
        let pos_a = priority_keys.iter().position(|k| *k == *key_a);
        let pos_b = priority_keys.iter().position(|k| *k == *key_b);

        match (pos_a, pos_b) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => key_a.cmp(key_b),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
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

    #[test]
    fn resolves_ctrl_j_on_search_screen() {
        let action = resolve_action(CurrentScreen::Search, ctrl_key('j'));
        assert!(
            matches!(action, Some(Action::NavigateDown)),
            "Ctrl+j on Search screen should return NavigateDown action"
        );
    }

    #[test]
    fn resolves_ctrl_k_on_search_screen() {
        let action = resolve_action(CurrentScreen::Search, ctrl_key('k'));
        assert!(
            matches!(action, Some(Action::NavigateUp)),
            "Ctrl+k on Search screen should return NavigateUp action"
        );
    }
}
