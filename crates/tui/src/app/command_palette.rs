//! Command palette data structures and fuzzy search logic.
//!
//! Responsibilities:
//! - Define CommandPaletteItem for action metadata
//! - Build catalog of all available actions
//! - Perform fuzzy search using simple string matching
//! - Filter items based on current screen context
//!
//! Does NOT handle:
//! - Does NOT handle input (handled by app::popups::command_palette)
//! - Does NOT render the palette (handled by ui::popup)

use crate::action::Action;
use crate::app::CurrentScreen;
use crate::input::keymap::{BindingScope, keybindings};

/// Maximum number of recent commands to track
const MAX_RECENT_COMMANDS: usize = 10;

/// An item in the command palette.
#[derive(Debug, Clone)]
pub struct CommandPaletteItem {
    /// Display name for the command
    pub name: String,
    /// Keyboard shortcut hint (e.g., "Ctrl+P", "r")
    pub shortcut: Option<String>,
    /// Detailed description of what the command does
    pub description: String,
    /// The action to execute when selected
    pub action: Action,
    /// Which sections/screens this command applies to
    pub scope: CommandScope,
    /// Whether this is a recent command
    pub is_recent: bool,
}

impl PartialEq for CommandPaletteItem {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.shortcut == other.shortcut
            && self.description == other.description
            && actions_equal(&self.action, &other.action)
            && self.scope == other.scope
            && self.is_recent == other.is_recent
    }
}

impl Eq for CommandPaletteItem {}

/// Scope for command palette items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandScope {
    /// Available globally
    Global,
    /// Available only on specific screens (stored as bit flags)
    Screens(u32),
    /// Available everywhere but context-aware priority
    ContextAware,
}

/// State for the command palette.
pub struct CommandPaletteState {
    /// All available commands
    all_commands: Vec<CommandPaletteItem>,
    /// Recent commands (most recent first)
    pub recent_commands: Vec<Action>,
}

impl CommandPaletteState {
    /// Create new command palette state with all available commands.
    pub fn new() -> Self {
        Self {
            all_commands: build_command_catalog(),
            recent_commands: Vec::new(),
        }
    }

    /// Search commands with fuzzy matching.
    pub fn search(&self, query: &str, current_screen: CurrentScreen) -> Vec<CommandPaletteItem> {
        if query.is_empty() {
            // Return recent commands first, then context-relevant commands
            return self.get_default_items(current_screen);
        }

        let query_lower = query.to_lowercase();
        let mut scored_items: Vec<(i32, CommandPaletteItem)> = self
            .all_commands
            .iter()
            .filter(|item| item.is_available_on(current_screen))
            .filter_map(|item| {
                // Simple fuzzy matching algorithm
                let name_score = fuzzy_match(&item.name, &query_lower);
                let desc_score = fuzzy_match(&item.description, &query_lower);

                // Take the best score
                let score = name_score.max(desc_score);

                // Only include items with positive score
                if score > 0 {
                    Some((score, item.clone()))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score (higher is better)
        scored_items.sort_by(|a, b| b.0.cmp(&a.0));

        scored_items.into_iter().map(|(_, item)| item).collect()
    }

    /// Get default items when no search query (recent + context-aware).
    fn get_default_items(&self, current_screen: CurrentScreen) -> Vec<CommandPaletteItem> {
        let mut items = Vec::new();

        // Add recent commands first
        for recent_action in &self.recent_commands {
            if let Some(mut item) = self.find_item_by_action(recent_action) {
                if item.is_available_on(current_screen) {
                    item.is_recent = true;
                    items.push(item);
                }
            }
        }

        // Add context-relevant commands (excluding already added recent ones)
        for item in &self.all_commands {
            if item.is_available_on(current_screen)
                && !items.iter().any(|i| actions_equal(&i.action, &item.action))
            {
                items.push(item.clone());
            }
        }

        items
    }

    /// Find an item by its action.
    fn find_item_by_action(&self, action: &Action) -> Option<CommandPaletteItem> {
        self.all_commands
            .iter()
            .find(|item| actions_equal(&item.action, action))
            .cloned()
    }

    /// Record a command as recently used.
    pub fn record_command(&mut self, action: &Action) {
        // Remove if already exists to move to front
        self.recent_commands.retain(|a| !actions_equal(a, action));

        self.recent_commands.insert(0, action.clone());
        self.recent_commands.truncate(MAX_RECENT_COMMANDS);
    }
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple fuzzy matching algorithm.
/// Returns a score based on how well the query matches the text.
/// Higher scores are better matches.
fn fuzzy_match(text: &str, query: &str) -> i32 {
    let text_lower = text.to_lowercase();
    let text_chars: Vec<char> = text_lower.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    let mut score = 0;
    let mut text_idx = 0;
    let mut query_idx = 0;
    let mut consecutive_bonus = 0;

    // Check for exact substring match (highest score)
    if text_lower.contains(query) {
        score += 100;
        // Bonus for matching at start
        if text_lower.starts_with(query) {
            score += 50;
        }
        return score;
    }

    // Fuzzy match: check if all query characters appear in order
    while text_idx < text_chars.len() && query_idx < query_chars.len() {
        if text_chars[text_idx] == query_chars[query_idx] {
            score += 10;
            // Bonus for consecutive matches
            score += consecutive_bonus;
            consecutive_bonus = 5;
            query_idx += 1;
        } else {
            consecutive_bonus = 0;
        }
        text_idx += 1;
    }

    // Only return score if all query characters were matched
    if query_idx == query_chars.len() {
        score
    } else {
        0
    }
}

/// Check if two actions are equal by discriminant.
fn actions_equal(a: &Action, b: &Action) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

impl CommandPaletteItem {
    /// Check if this command is available on the given screen.
    pub fn is_available_on(&self, screen: CurrentScreen) -> bool {
        match self.scope {
            CommandScope::Global => true,
            CommandScope::Screens(screen_flags) => {
                let screen_bit = screen_to_bit(screen);
                (screen_flags & screen_bit) != 0
            }
            CommandScope::ContextAware => true,
        }
    }
}

/// Convert a screen to a bit flag.
fn screen_to_bit(screen: CurrentScreen) -> u32 {
    match screen {
        CurrentScreen::Search => 1 << 0,
        CurrentScreen::Indexes => 1 << 1,
        CurrentScreen::Cluster => 1 << 2,
        CurrentScreen::Jobs => 1 << 3,
        CurrentScreen::JobInspect => 1 << 4,
        CurrentScreen::Health => 1 << 5,
        CurrentScreen::License => 1 << 6,
        CurrentScreen::Kvstore => 1 << 7,
        CurrentScreen::SavedSearches => 1 << 8,
        CurrentScreen::Macros => 1 << 9,
        CurrentScreen::InternalLogs => 1 << 10,
        CurrentScreen::Apps => 1 << 11,
        CurrentScreen::Users => 1 << 12,
        CurrentScreen::Roles => 1 << 13,
        CurrentScreen::SearchPeers => 1 << 14,
        CurrentScreen::Inputs => 1 << 15,
        CurrentScreen::Configs => 1 << 16,
        CurrentScreen::Settings => 1 << 17,
        CurrentScreen::Overview => 1 << 18,
        CurrentScreen::MultiInstance => 1 << 19,
        CurrentScreen::FiredAlerts => 1 << 20,
        CurrentScreen::Forwarders => 1 << 21,
        CurrentScreen::Lookups => 1 << 22,
        CurrentScreen::Audit => 1 << 23,
        CurrentScreen::Dashboards => 1 << 24,
        CurrentScreen::DataModels => 1 << 25,
        CurrentScreen::WorkloadManagement => 1 << 26,
        CurrentScreen::Shc => 1 << 27,
    }
}

/// Build the complete catalog of available commands from keybindings.
fn build_command_catalog() -> Vec<CommandPaletteItem> {
    let mut commands = Vec::new();

    // Build from keybindings
    for binding in keybindings() {
        if let Some(action) = binding.action {
            let scope = match binding.scope {
                BindingScope::Global => CommandScope::Global,
                BindingScope::Screen(screen) => CommandScope::Screens(screen_to_bit(screen)),
            };

            commands.push(CommandPaletteItem {
                name: binding.description.to_string(),
                shortcut: Some(binding.keys.to_string()),
                description: format!("{} ({})", binding.description, binding.keys),
                action,
                scope,
                is_recent: false,
            });
        }
    }

    // Add screen navigation commands explicitly
    commands.extend(build_screen_navigation_commands());

    // Deduplicate by action
    let mut seen = std::collections::HashSet::new();
    commands
        .into_iter()
        .filter(|item| seen.insert(std::mem::discriminant(&item.action)))
        .collect()
}

/// Build commands for navigating to each screen.
fn build_screen_navigation_commands() -> Vec<CommandPaletteItem> {
    use CurrentScreen::*;

    vec![
        (
            Search,
            "Go to Search",
            "Open search screen",
            Action::SwitchToSearch,
        ),
        (
            Indexes,
            "Go to Indexes",
            "View and manage indexes",
            Action::LoadIndexes {
                count: 30,
                offset: 0,
            },
        ),
        (
            Jobs,
            "Go to Jobs",
            "View search jobs",
            Action::LoadJobs {
                count: 30,
                offset: 0,
            },
        ),
        (
            Cluster,
            "Go to Cluster",
            "View cluster information",
            Action::LoadClusterInfo,
        ),
        (
            Health,
            "Go to Health",
            "View health status",
            Action::LoadHealth,
        ),
        (
            License,
            "Go to License",
            "View license information",
            Action::LoadLicense,
        ),
        (
            Kvstore,
            "Go to KVStore",
            "View KVStore status",
            Action::LoadKvstore,
        ),
        (
            SavedSearches,
            "Go to Saved Searches",
            "Manage saved searches",
            Action::LoadSavedSearches,
        ),
        (Macros, "Go to Macros", "Manage macros", Action::LoadMacros),
        (
            InternalLogs,
            "Go to Internal Logs",
            "View internal logs",
            Action::LoadInternalLogs {
                count: 100,
                earliest: "-15m".to_string(),
            },
        ),
        (
            Apps,
            "Go to Apps",
            "Manage apps",
            Action::LoadApps {
                count: 30,
                offset: 0,
            },
        ),
        (
            Users,
            "Go to Users",
            "Manage users",
            Action::LoadUsers {
                count: 30,
                offset: 0,
            },
        ),
        (
            Roles,
            "Go to Roles",
            "Manage roles",
            Action::LoadRoles {
                count: 30,
                offset: 0,
            },
        ),
        (
            SearchPeers,
            "Go to Search Peers",
            "View search peers",
            Action::LoadSearchPeers {
                count: 30,
                offset: 0,
            },
        ),
        (
            Inputs,
            "Go to Inputs",
            "Manage data inputs",
            Action::LoadInputs {
                count: 30,
                offset: 0,
            },
        ),
        (
            Configs,
            "Go to Configs",
            "View configuration files",
            Action::LoadConfigFiles,
        ),
        (
            FiredAlerts,
            "Go to Fired Alerts",
            "View fired alerts",
            Action::LoadFiredAlerts {
                count: 30,
                offset: 0,
            },
        ),
        (
            Forwarders,
            "Go to Forwarders",
            "View forwarders",
            Action::LoadForwarders {
                count: 30,
                offset: 0,
            },
        ),
        (
            Lookups,
            "Go to Lookups",
            "Manage lookup tables",
            Action::LoadLookups {
                count: 30,
                offset: 0,
            },
        ),
        (
            Audit,
            "Go to Audit",
            "View audit events",
            Action::LoadRecentAuditEvents { count: 50 },
        ),
        (
            Dashboards,
            "Go to Dashboards",
            "View dashboards",
            Action::LoadDashboards {
                count: 30,
                offset: 0,
            },
        ),
        (
            DataModels,
            "Go to Data Models",
            "View data models",
            Action::LoadDataModels {
                count: 30,
                offset: 0,
            },
        ),
        (
            WorkloadManagement,
            "Go to Workload",
            "Manage workload",
            Action::LoadWorkloadPools {
                count: 30,
                offset: 0,
            },
        ),
        (
            Shc,
            "Go to SHC",
            "View search head cluster",
            Action::LoadShcStatus,
        ),
        (
            Settings,
            "Go to Settings",
            "Application settings",
            Action::SwitchToSettingsScreen,
        ),
        (
            Overview,
            "Go to Overview",
            "Resource overview",
            Action::LoadOverview,
        ),
        (
            MultiInstance,
            "Go to Multi-Instance",
            "Multi-instance dashboard",
            Action::LoadMultiInstanceOverview,
        ),
    ]
    .into_iter()
    .map(|(_screen, name, desc, action)| CommandPaletteItem {
        name: name.to_string(),
        shortcut: None,
        description: desc.to_string(),
        action,
        scope: CommandScope::Global,
        is_recent: false,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_palette_state_new() {
        let state = CommandPaletteState::new();
        assert!(!state.all_commands.is_empty());
        assert!(state.recent_commands.is_empty());
    }

    #[test]
    fn test_search_empty_query_returns_items() {
        let state = CommandPaletteState::new();
        let items = state.search("", CurrentScreen::Search);
        assert!(!items.is_empty());
    }

    #[test]
    fn test_search_finds_commands() {
        let state = CommandPaletteState::new();
        let items = state.search("quit", CurrentScreen::Search);
        assert!(!items.is_empty());
        // Should find quit command
        assert!(items.iter().any(|i| i.name.to_lowercase().contains("quit")));
    }

    #[test]
    fn test_recent_commands_tracking() {
        let mut state = CommandPaletteState::new();
        let action = Action::Quit;

        state.record_command(&action);
        assert_eq!(state.recent_commands.len(), 1);
        assert!(actions_equal(&state.recent_commands[0], &action));

        // Recording same command again should move it to front, not duplicate
        state.record_command(&action);
        assert_eq!(state.recent_commands.len(), 1);
    }

    #[test]
    fn test_recent_commands_limit() {
        let mut state = CommandPaletteState::new();

        // Add more than max commands (using different action variants)
        // We cycle through a few different action types to ensure they're unique
        let actions = [
            Action::NextScreen,
            Action::PreviousScreen,
            Action::NavigateDown,
            Action::NavigateUp,
            Action::PageDown,
            Action::PageUp,
            Action::GoToTop,
            Action::GoToBottom,
            Action::NextFocus,
            Action::PreviousFocus,
            Action::ToggleFocusMode,
            Action::CycleTheme,
            Action::ToggleSortDirection,
            Action::ToggleSearchMode,
            Action::InspectJob,
        ];

        for i in 0..MAX_RECENT_COMMANDS + 5 {
            state.record_command(&actions[i % actions.len()]);
        }

        assert_eq!(
            state.recent_commands.len(),
            MAX_RECENT_COMMANDS.min(actions.len())
        );
    }

    #[test]
    fn test_recent_commands_in_default_items() {
        let mut state = CommandPaletteState::new();
        let action = Action::Quit;

        state.record_command(&action);
        let items = state.get_default_items(CurrentScreen::Search);

        // First item should be marked as recent
        assert!(items[0].is_recent);
    }

    #[test]
    fn test_is_available_on_global() {
        let item = CommandPaletteItem {
            name: "Test".to_string(),
            shortcut: None,
            description: "Test".to_string(),
            action: Action::Quit,
            scope: CommandScope::Global,
            is_recent: false,
        };

        assert!(item.is_available_on(CurrentScreen::Search));
        assert!(item.is_available_on(CurrentScreen::Jobs));
    }

    #[test]
    fn test_is_available_on_screen() {
        let item = CommandPaletteItem {
            name: "Test".to_string(),
            shortcut: None,
            description: "Test".to_string(),
            action: Action::Quit,
            scope: CommandScope::Screens(screen_to_bit(CurrentScreen::Search)),
            is_recent: false,
        };

        assert!(item.is_available_on(CurrentScreen::Search));
        assert!(!item.is_available_on(CurrentScreen::Jobs));
    }

    #[test]
    fn test_actions_equal_same_variant() {
        let a = Action::Quit;
        let b = Action::Quit;
        assert!(actions_equal(&a, &b));
    }

    #[test]
    fn test_actions_equal_different_variants() {
        let a = Action::Quit;
        let b = Action::OpenHelpPopup;
        assert!(!actions_equal(&a, &b));
    }

    #[test]
    fn test_screen_to_bit_unique() {
        let screens = [
            CurrentScreen::Search,
            CurrentScreen::Indexes,
            CurrentScreen::Cluster,
            CurrentScreen::Jobs,
            CurrentScreen::JobInspect,
            CurrentScreen::Health,
            CurrentScreen::License,
            CurrentScreen::Kvstore,
            CurrentScreen::SavedSearches,
            CurrentScreen::Macros,
            CurrentScreen::InternalLogs,
            CurrentScreen::Apps,
            CurrentScreen::Users,
            CurrentScreen::Roles,
            CurrentScreen::SearchPeers,
            CurrentScreen::Inputs,
            CurrentScreen::Configs,
            CurrentScreen::Settings,
            CurrentScreen::Overview,
            CurrentScreen::MultiInstance,
            CurrentScreen::FiredAlerts,
            CurrentScreen::Forwarders,
            CurrentScreen::Lookups,
            CurrentScreen::Audit,
            CurrentScreen::Dashboards,
            CurrentScreen::DataModels,
            CurrentScreen::WorkloadManagement,
            CurrentScreen::Shc,
        ];

        let mut bits = std::collections::HashSet::new();
        for screen in screens {
            let bit = screen_to_bit(screen);
            assert!(bits.insert(bit), "Duplicate bit for screen {:?}", screen);
        }
    }

    #[test]
    fn test_fuzzy_match_exact() {
        let score = fuzzy_match("Quit Application", "quit");
        assert!(score > 0);
        // Exact match should have high score
        assert!(score >= 100);
    }

    #[test]
    fn test_fuzzy_match_prefix() {
        let exact_score = fuzzy_match("Search", "search");
        let prefix_score = fuzzy_match("Search Results", "search");

        assert!(exact_score > 0);
        assert!(prefix_score > 0);
        // Exact match should score higher or equal
        assert!(exact_score >= prefix_score);
    }

    #[test]
    fn test_fuzzy_match_fuzzy() {
        let score = fuzzy_match("Go to Settings", "gts");
        assert!(score > 0); // Should match all letters in order
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        let score = fuzzy_match("Quit", "xyz");
        assert_eq!(score, 0);
    }

    #[test]
    fn test_fuzzy_match_consecutive_bonus() {
        let consecutive = fuzzy_match("abcdef", "abc");
        let non_consecutive = fuzzy_match("axbxcx", "abc");

        assert!(consecutive > non_consecutive);
    }
}
