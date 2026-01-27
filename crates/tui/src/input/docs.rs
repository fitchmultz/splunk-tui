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

use crate::input::keymap::{Section, keybindings};

pub(crate) fn render_markdown() -> String {
    let mut out = String::new();

    out.push_str("### Navigation\n\n");
    for (keys, description) in unique_entries(Section::Global) {
        out.push_str(&format!("- `{}`: {}\n", keys, description));
    }

    out.push_str("\n### Screen Specific Shortcuts\n\n");

    for section in screen_sections() {
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

fn screen_sections() -> Vec<Section> {
    vec![
        Section::Search,
        Section::Jobs,
        Section::JobDetails,
        Section::Indexes,
        Section::Cluster,
        Section::Health,
        Section::SavedSearches,
        Section::InternalLogs,
        Section::Apps,
        Section::Users,
        Section::Settings,
    ]
}

fn section_heading(section: Section) -> &'static str {
    match section {
        Section::Search => "Search Screen",
        Section::Jobs => "Jobs Screen",
        Section::JobDetails => "Job Details (Inspect) Screen",
        Section::Indexes => "Indexes Screen",
        Section::Cluster => "Cluster Screen",
        Section::Health => "Health Screen",
        Section::SavedSearches => "Saved Searches Screen",
        Section::InternalLogs => "Internal Logs Screen",
        Section::Apps => "Apps Screen",
        Section::Users => "Users Screen",
        Section::Settings => "Settings Screen",
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
