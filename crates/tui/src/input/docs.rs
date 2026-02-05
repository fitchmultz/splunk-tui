//! Documentation rendering helpers for TUI keybindings.
//!
//! Responsibilities:
//! - Render the keybinding catalog into Markdown for docs/usage.md.
//!
//! Non-responsibilities:
//! - Reading or writing files (handled by generator binaries).
//! - Determining where generated blocks live in documentation files.
//!
//! Invariants:
//! - Output is deterministic and based on the keybinding catalog.

use std::collections::BTreeSet;

use crate::input::keymap::{Section, keybindings, sections_in_order};

pub(crate) fn render_markdown() -> String {
    let mut out = String::new();

    out.push_str("### Navigation\n\n");
    for (keys, description) in unique_entries(Section::Global) {
        out.push_str(&format!("- `{}`: {}\n", keys, description));
    }

    out.push_str("\n### Screen Specific Shortcuts\n\n");

    for section in screen_sections() {
        // Special handling for Search screen to document input modes
        if section == Section::Search {
            out.push_str(&render_search_screen_docs());
            continue;
        }

        let entries = unique_entries(section);
        if entries.is_empty() {
            continue;
        }
        out.push_str(&format!("#### {}\n", section_heading(section)));
        for (keys, description) in entries {
            out.push_str(&format!("- `{}`: {}\n", keys, description));
        }
        out.push('\n');
    }

    out.trim_end().to_string()
}

/// Render Search screen documentation with input mode information.
fn render_search_screen_docs() -> String {
    let mut out = String::new();
    out.push_str("#### Search Screen\n\n");
    out.push_str("The Search screen has two input modes that affect how keys are handled:\n\n");
    out.push_str(
        "**QueryFocused mode** (default): Type your search query. Printable characters (including `q`, `?`, digits) are inserted into the query. Use `Tab` to switch to ResultsFocused mode.\n\n"
    );
    out.push_str(
        "**ResultsFocused mode**: Navigate and control the application. Global shortcuts like `q` (quit) and `?` (help) work in this mode. Use `Tab` or `Esc` to return to QueryFocused mode.\n\n"
    );

    // Add keybindings from the keymap
    for (keys, description) in unique_entries(Section::Search) {
        out.push_str(&format!("- `{}`: {}\n", keys, description));
    }
    out.push('\n');

    out
}

fn screen_sections() -> Vec<Section> {
    sections_in_order()
        .iter()
        .copied()
        .filter(|s| *s != Section::Global)
        .collect()
}

fn section_heading(section: Section) -> &'static str {
    match section {
        Section::Search => "Search Screen",
        Section::Jobs => "Jobs Screen",
        Section::JobDetails => "Job Details (Inspect) Screen",
        Section::Indexes => "Indexes Screen",
        Section::Cluster => "Cluster Screen",
        Section::Health => "Health Screen",
        Section::License => "License Screen",
        Section::Kvstore => "KVStore Screen",
        Section::SavedSearches => "Saved Searches Screen",
        Section::InternalLogs => "Internal Logs Screen",
        Section::Apps => "Apps Screen",
        Section::Users => "Users Screen",
        Section::Roles => "Roles Screen",
        Section::SearchPeers => "Search Peers Screen",
        Section::Inputs => "Data Inputs Screen",
        Section::Configs => "Configuration Files Screen",
        Section::FiredAlerts => "Fired Alerts Screen",
        Section::Forwarders => "Forwarders Screen",
        Section::Lookups => "Lookups Screen",
        Section::Audit => "Audit Events Screen",
        Section::Dashboards => "Dashboards Screen",
        Section::DataModels => "Data Models Screen",
        Section::Workload => "Workload Management Screen",
        Section::Shc => "SHC Screen",
        Section::Settings => "Settings Screen",
        Section::Overview => "Overview Screen",
        Section::MultiInstance => "Multi-Instance Dashboard Screen",
        Section::Global => "Global Keys",
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

#[cfg(test)]
mod tests {
    use super::render_markdown;

    #[test]
    fn render_markdown_includes_navigation_and_search_sections() {
        let markdown = render_markdown();

        assert!(markdown.contains("### Navigation"));
        assert!(markdown.contains("#### Search Screen"));
        assert!(markdown.contains("- `?`: Help"));
        assert!(markdown.contains("- `Enter`: Run search"));
    }

    #[test]
    fn render_markdown_is_trimmed() {
        let markdown = render_markdown();
        assert!(!markdown.ends_with('\n'));
    }
}
