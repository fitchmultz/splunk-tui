//! Help popup rendering from centralized keybindings.
//!
//! Responsibilities:
//! - Convert keybinding metadata into a human-readable help string.
//!
//! Non-responsibilities:
//! - Mutating application state.
//! - Owning keybinding definitions (delegated to keymap).
//!
//! Invariants:
//! - Rendering order is stable across runs for snapshot determinism.

use std::collections::BTreeSet;

use crate::input::keymap::{Section, keybindings};

pub(crate) fn help_text() -> String {
    let order = [
        Section::Global,
        Section::Search,
        Section::Jobs,
        Section::JobDetails,
        Section::Indexes,
        Section::Cluster,
        Section::Health,
        Section::License,
        Section::Kvstore,
        Section::SavedSearches,
        Section::InternalLogs,
        Section::Apps,
        Section::Users,
        Section::Settings,
    ];

    let mut out = String::new();
    for section in order {
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
        Section::InternalLogs => "Internal Logs Screen:",
        Section::Apps => "Apps Screen:",
        Section::Users => "Users Screen:",
        Section::SearchPeers => "Search Peers Screen:",
        Section::Inputs => "Data Inputs Screen:",
        Section::Settings => "Settings Screen:",
        Section::Overview => "Overview Screen:",
    }
}
