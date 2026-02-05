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

use crate::input::keymap::{Section, keybindings, sections_in_order};

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
}
