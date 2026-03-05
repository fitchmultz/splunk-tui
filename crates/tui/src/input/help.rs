//! Help popup rendering from centralized keybindings.
//!
//! Responsibilities:
//! - Convert keybinding metadata into a human-readable help string.
//! - Generate context-aware help prioritized by current screen and mode.
//!
//! Does NOT handle:
//! - Mutating application state.
//! - Owning keybinding definitions (delegated to keymap).
//!
//! Invariants:
//! - Rendering order is stable across runs for snapshot determinism.
//! - Context-aware help shows current screen bindings first.

use std::collections::BTreeSet;

use crate::app::{CurrentScreen, state::SearchInputMode};
use crate::input::keymap::overrides::get_effective_key_display;
use crate::input::keymap::{
    Section, get_priority_keys, keybindings, screen_to_section, sections_in_order,
};

pub(crate) fn help_text() -> String {
    let order = sections_in_order();

    let mut out = String::new();
    for &section in order {
        let entries = unique_entries(section);
        if entries.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(section_title(section));
        out.push('\n');
        let max_key_len = entries
            .iter()
            .map(|(keys, _)| keys.chars().count())
            .max()
            .unwrap_or(0);
        for (keys, description) in entries {
            let padding = max_key_len.saturating_sub(keys.chars().count()) + 2;
            out.push_str("  ");
            out.push_str(keys);
            out.push_str(&" ".repeat(padding));
            out.push_str(description);
            out.push('\n');
        }
    }
    out
}

/// Generate context-aware help text prioritized by current screen.
///
/// Shows the most relevant keybindings first based on:
/// 1. Current screen's section (top priority bindings first)
/// 2. Global Keys section (always visible)
/// 3. Other screens (collapsed view with limited bindings)
///
/// Respects keybinding overrides to show user's custom keys.
pub fn contextual_help_text(screen: CurrentScreen, input_mode: Option<SearchInputMode>) -> String {
    let current_section = screen_to_section(screen);
    let priority_keys = get_priority_keys(screen);

    let mut out = String::new();

    // 1. Current screen section first (prioritized with override-aware keys)
    let current_entries =
        prioritized_entries_for_section(current_section, priority_keys, screen, input_mode);
    if !current_entries.is_empty() {
        out.push_str(section_title(current_section));
        out.push('\n');
        let max_key_len = current_entries
            .iter()
            .map(|(keys, _)| keys.chars().count())
            .max()
            .unwrap_or(0);
        for (keys, description) in current_entries {
            let padding = max_key_len.saturating_sub(keys.chars().count()) + 2;
            out.push_str("  ");
            out.push_str(&keys);
            out.push_str(&" ".repeat(padding));
            out.push_str(description);
            out.push('\n');
        }
    }

    // 2. Global Keys section (always visible with override-aware keys)
    let global_entries = entries_with_overrides(Section::Global);
    if !global_entries.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(section_title(Section::Global));
        out.push('\n');
        let max_key_len = global_entries
            .iter()
            .map(|(keys, _)| keys.chars().count())
            .max()
            .unwrap_or(0);
        for (keys, description) in global_entries {
            let padding = max_key_len.saturating_sub(keys.chars().count()) + 2;
            out.push_str("  ");
            out.push_str(&keys);
            out.push_str(&" ".repeat(padding));
            out.push_str(description);
            out.push('\n');
        }
    }

    // 3. Other screens (collapsed view - limited to top 3 bindings per section)
    out.push('\n');
    out.push_str("─────────────────────────────────────\n");
    out.push_str("Other screens (press j/k to scroll):\n");
    out.push_str("─────────────────────────────────────\n");

    for &section in sections_in_order() {
        if section == current_section || section == Section::Global {
            continue;
        }

        let entries = entries_with_overrides(section);
        if entries.is_empty() {
            continue;
        }

        out.push('\n');
        out.push_str(section_title(section));
        out.push('\n');

        // Limit to top 3 bindings for collapsed view
        let max_key_len = entries
            .iter()
            .take(3)
            .map(|(keys, _)| keys.chars().count())
            .max()
            .unwrap_or(0);

        for (keys, description) in entries.into_iter().take(3) {
            let padding = max_key_len.saturating_sub(keys.chars().count()) + 2;
            out.push_str("  ");
            out.push_str(&keys);
            out.push_str(&" ".repeat(padding));
            out.push_str(description);
            out.push('\n');
        }
    }

    out
}

/// Get prioritized entries for a section with override-aware key labels.
fn prioritized_entries_for_section(
    section: Section,
    priority_keys: &[&'static str],
    screen: CurrentScreen,
    input_mode: Option<SearchInputMode>,
) -> Vec<(String, &'static str)> {
    let mut entries: Vec<(String, &'static str)> = keybindings()
        .into_iter()
        .filter(|b| b.section == section)
        .filter(|b| is_binding_relevant(b, screen, input_mode))
        .map(|b| {
            let key_display = get_effective_key_display_for_binding(&b);
            (key_display, b.description)
        })
        .collect();

    // Remove duplicates based on description (keeps first occurrence)
    let mut seen_descriptions = BTreeSet::new();
    entries.retain(|(_, desc)| seen_descriptions.insert(*desc));

    // Sort by priority order
    entries.sort_by(|(key_a, _), (key_b, _)| {
        let pos_a = priority_keys.iter().position(|k| *k == key_a.as_str());
        let pos_b = priority_keys.iter().position(|k| *k == key_b.as_str());

        match (pos_a, pos_b) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => key_a.cmp(key_b),
        }
    });

    entries
}

/// Get entries for a section with override-aware key labels.
fn entries_with_overrides(section: Section) -> Vec<(String, &'static str)> {
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();

    for binding in keybindings() {
        if binding.section != section {
            continue;
        }

        let key_display = get_effective_key_display_for_binding(&binding);
        let key = (key_display.clone(), binding.description);

        if seen.insert(key.clone()) {
            entries.push((key_display, binding.description));
        }
    }

    entries
}

/// Get override-aware key display for a binding.
fn get_effective_key_display_for_binding(binding: &crate::input::keymap::Keybinding) -> String {
    if let Some(ref action) = binding.action {
        get_effective_key_display(action.clone(), binding.keys)
    } else {
        binding.keys.to_string()
    }
}

/// Check if a binding is relevant for current screen/mode context.
fn is_binding_relevant(
    binding: &crate::input::keymap::Keybinding,
    screen: CurrentScreen,
    input_mode: Option<SearchInputMode>,
) -> bool {
    match (screen, input_mode) {
        (CurrentScreen::Search, Some(SearchInputMode::QueryFocused)) => {
            // In query mode, prioritize query-related bindings and global navigation
            // Exclude pure navigation bindings that don't make sense in query mode
            binding.scope_applies(screen)
        }
        _ => binding.scope_applies(screen),
    }
}

fn unique_entries(section: Section) -> Vec<(&'static str, &'static str)> {
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for binding in keybindings() {
        if binding.section != section {
            continue;
        }
        let key = (binding.keys, binding.description);
        if seen.insert(key) {
            entries.push(key);
        }
    }
    entries
}

fn section_title(section: Section) -> &'static str {
    match section {
        Section::Global => "Global Keys:",
        Section::Search => "Search Screen:",
        Section::Jobs => "Jobs Screen:",
        Section::JobDetails => "Job Details Screen:",
        Section::Indexes => "Indexes Screen:",
        Section::Cluster => "Cluster Screen:",
        Section::Health => "Health Screen:",
        Section::License => "License Screen:",
        Section::Kvstore => "KVStore Screen:",
        Section::SavedSearches => "Saved Searches Screen:",
        Section::Macros => "Macros Screen:",
        Section::InternalLogs => "Internal Logs Screen:",
        Section::Apps => "Apps Screen:",
        Section::Users => "Users Screen:",
        Section::Roles => "Roles Screen:",
        Section::SearchPeers => "Search Peers Screen:",
        Section::Inputs => "Data Inputs Screen:",
        Section::Configs => "Configuration Files Screen:",
        Section::FiredAlerts => "Fired Alerts Screen:",
        Section::Forwarders => "Forwarders Screen:",
        Section::Lookups => "Lookups Screen:",
        Section::Audit => "Audit Events Screen:",
        Section::Dashboards => "Dashboards Screen:",
        Section::DataModels => "Data Models Screen:",
        Section::Workload => "Workload Management Screen:",
        Section::Settings => "Settings Screen:",
        Section::Overview => "Overview Screen:",
        Section::MultiInstance => "Multi-Instance Dashboard Screen:",
        Section::Shc => "SHC Screen:",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_text_includes_data_models_section() {
        let help = help_text();
        assert!(
            help.contains("Data Models Screen:"),
            "Help should include Data Models section"
        );
    }

    #[test]
    fn help_text_includes_workload_section() {
        let help = help_text();
        assert!(
            help.contains("Workload Management Screen:"),
            "Help should include Workload Management section"
        );
    }

    #[test]
    fn help_text_includes_shc_section() {
        let help = help_text();
        assert!(
            help.contains("SHC Screen:"),
            "Help should include SHC section"
        );
    }

    #[test]
    fn help_text_includes_global_section() {
        let help = help_text();
        assert!(
            help.contains("Global Keys:"),
            "Help should include Global Keys section"
        );
    }

    #[test]
    fn help_text_includes_macros_section() {
        let help = help_text();
        assert!(
            help.contains("Macros Screen:"),
            "Help should include Macros section"
        );
    }

    #[test]
    fn help_text_includes_error_details_keybinding() {
        let help = help_text();
        assert!(
            help.contains("Show error details (when an error is present)"),
            "Help should include 'e' keybinding for error details"
        );
    }
}
