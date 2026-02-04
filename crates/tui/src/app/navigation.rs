//! Navigation helpers for the TUI app.
//!
//! Responsibilities:
//! - Handle item navigation (next/previous)
//! - Handle page navigation (page up/down)
//! - Handle jump navigation (top/bottom)
//!
//! Non-responsibilities:
//! - Does NOT handle screen switching (handled by actions)
//! - Does NOT handle input events

use crate::app::App;
use crate::app::state::CurrentScreen;

impl App {
    // Navigation helpers
    pub(crate) fn next_item(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                if !self.search_results.is_empty() {
                    let max_offset = self.search_results.len().saturating_sub(1);
                    if self.search_scroll_offset < max_offset {
                        self.search_scroll_offset += 1;
                    }
                }
            }
            CurrentScreen::Jobs => {
                let len = self.filtered_jobs_len();
                if len > 0 {
                    let i = self.jobs_state.selected().unwrap_or(0);
                    if i < len.saturating_sub(1) {
                        self.jobs_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::Indexes => {
                if let Some(indexes) = &self.indexes {
                    let i = self.indexes_state.selected().unwrap_or(0);
                    if i < indexes.len().saturating_sub(1) {
                        self.indexes_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::SavedSearches => {
                if let Some(searches) = &self.saved_searches {
                    let i = self.saved_searches_state.selected().unwrap_or(0);
                    if i < searches.len().saturating_sub(1) {
                        self.saved_searches_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::InternalLogs => {
                if let Some(logs) = &self.internal_logs {
                    let i = self.internal_logs_state.selected().unwrap_or(0);
                    if i < logs.len().saturating_sub(1) {
                        self.internal_logs_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::Apps => {
                if let Some(apps) = &self.apps {
                    let i = self.apps_state.selected().unwrap_or(0);
                    if i < apps.len().saturating_sub(1) {
                        self.apps_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::Users => {
                if let Some(users) = &self.users {
                    let i = self.users_state.selected().unwrap_or(0);
                    if i < users.len().saturating_sub(1) {
                        self.users_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::Cluster => {
                if self.cluster_view_mode == crate::app::state::ClusterViewMode::Peers
                    && let Some(peers) = &self.cluster_peers
                {
                    let i = self.cluster_peers_state.selected().unwrap_or(0);
                    if i < peers.len().saturating_sub(1) {
                        self.cluster_peers_state.select(Some(i + 1));
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn previous_item(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                self.search_scroll_offset = self.search_scroll_offset.saturating_sub(1);
            }
            CurrentScreen::Jobs => {
                let i = self.jobs_state.selected().unwrap_or(0);
                if i > 0 {
                    self.jobs_state.select(Some(i - 1));
                }
            }
            CurrentScreen::Indexes => {
                let i = self.indexes_state.selected().unwrap_or(0);
                if i > 0 {
                    self.indexes_state.select(Some(i - 1));
                }
            }
            CurrentScreen::SavedSearches => {
                let i = self.saved_searches_state.selected().unwrap_or(0);
                if i > 0 {
                    self.saved_searches_state.select(Some(i - 1));
                }
            }
            CurrentScreen::InternalLogs => {
                let i = self.internal_logs_state.selected().unwrap_or(0);
                if i > 0 {
                    self.internal_logs_state.select(Some(i - 1));
                }
            }
            CurrentScreen::Apps => {
                let i = self.apps_state.selected().unwrap_or(0);
                if i > 0 {
                    self.apps_state.select(Some(i - 1));
                }
            }
            CurrentScreen::Users => {
                let i = self.users_state.selected().unwrap_or(0);
                if i > 0 {
                    self.users_state.select(Some(i - 1));
                }
            }
            CurrentScreen::Cluster => {
                if self.cluster_view_mode == crate::app::state::ClusterViewMode::Peers {
                    let i = self.cluster_peers_state.selected().unwrap_or(0);
                    if i > 0 {
                        self.cluster_peers_state.select(Some(i - 1));
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn next_page(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                // Clamp offset to prevent scrolling past the end
                let max_offset = self.search_results.len().saturating_sub(1);
                self.search_scroll_offset =
                    self.search_scroll_offset.saturating_add(10).min(max_offset);
            }
            CurrentScreen::Jobs => {
                let len = self.filtered_jobs_len();
                if len > 0 {
                    let i = self.jobs_state.selected().unwrap_or(0);
                    self.jobs_state
                        .select(Some((i.saturating_add(10)).min(len - 1)));
                }
            }
            CurrentScreen::Indexes => {
                if let Some(indexes) = &self.indexes {
                    let i = self.indexes_state.selected().unwrap_or(0);
                    self.indexes_state
                        .select(Some((i.saturating_add(10)).min(indexes.len() - 1)));
                }
            }
            CurrentScreen::SavedSearches => {
                if let Some(searches) = &self.saved_searches {
                    let i = self.saved_searches_state.selected().unwrap_or(0);
                    self.saved_searches_state
                        .select(Some((i.saturating_add(10)).min(searches.len() - 1)));
                }
            }
            CurrentScreen::InternalLogs => {
                if let Some(logs) = &self.internal_logs {
                    let i = self.internal_logs_state.selected().unwrap_or(0);
                    self.internal_logs_state
                        .select(Some((i.saturating_add(10)).min(logs.len() - 1)));
                }
            }
            CurrentScreen::Apps => {
                if let Some(apps) = &self.apps {
                    let i = self.apps_state.selected().unwrap_or(0);
                    self.apps_state
                        .select(Some((i.saturating_add(10)).min(apps.len() - 1)));
                }
            }
            CurrentScreen::Cluster => {
                if self.cluster_view_mode == crate::app::state::ClusterViewMode::Peers
                    && let Some(peers) = &self.cluster_peers
                    && !peers.is_empty()
                {
                    let i = self.cluster_peers_state.selected().unwrap_or(0);
                    self.cluster_peers_state
                        .select(Some((i.saturating_add(10)).min(peers.len() - 1)));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn previous_page(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                // saturating_sub already prevents going below 0
                self.search_scroll_offset = self.search_scroll_offset.saturating_sub(10);
            }
            CurrentScreen::Jobs => {
                let i = self.jobs_state.selected().unwrap_or(0);
                self.jobs_state.select(Some(i.saturating_sub(10)));
            }
            CurrentScreen::Indexes => {
                let i = self.indexes_state.selected().unwrap_or(0);
                self.indexes_state.select(Some(i.saturating_sub(10)));
            }
            CurrentScreen::SavedSearches => {
                let i = self.saved_searches_state.selected().unwrap_or(0);
                self.saved_searches_state.select(Some(i.saturating_sub(10)));
            }
            CurrentScreen::InternalLogs => {
                let i = self.internal_logs_state.selected().unwrap_or(0);
                self.internal_logs_state.select(Some(i.saturating_sub(10)));
            }
            CurrentScreen::Apps => {
                let i = self.apps_state.selected().unwrap_or(0);
                self.apps_state.select(Some(i.saturating_sub(10)));
            }
            CurrentScreen::Cluster => {
                if self.cluster_view_mode == crate::app::state::ClusterViewMode::Peers {
                    let i = self.cluster_peers_state.selected().unwrap_or(0);
                    self.cluster_peers_state.select(Some(i.saturating_sub(10)));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn go_to_top(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                self.search_scroll_offset = 0;
            }
            CurrentScreen::Jobs => {
                if self.filtered_jobs_len() > 0 {
                    self.jobs_state.select(Some(0));
                }
            }
            CurrentScreen::Indexes => {
                self.indexes_state.select(Some(0));
            }
            CurrentScreen::SavedSearches => {
                self.saved_searches_state.select(Some(0));
            }
            CurrentScreen::InternalLogs => {
                self.internal_logs_state.select(Some(0));
            }
            CurrentScreen::Apps => {
                self.apps_state.select(Some(0));
            }
            CurrentScreen::Users => {
                self.users_state.select(Some(0));
            }
            CurrentScreen::Cluster => {
                if self.cluster_view_mode == crate::app::state::ClusterViewMode::Peers {
                    self.cluster_peers_state.select(Some(0));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn go_to_bottom(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                // Scroll to the last valid page (offset such that at least one result is visible)
                if !self.search_results.is_empty() {
                    self.search_scroll_offset = self.search_results.len().saturating_sub(1);
                } else {
                    self.search_scroll_offset = 0;
                }
            }
            CurrentScreen::Jobs => {
                let len = self.filtered_jobs_len();
                if len > 0 {
                    self.jobs_state.select(Some(len.saturating_sub(1)));
                }
            }
            CurrentScreen::Indexes => {
                if let Some(indexes) = &self.indexes {
                    self.indexes_state
                        .select(Some(indexes.len().saturating_sub(1)));
                }
            }
            CurrentScreen::SavedSearches => {
                if let Some(searches) = &self.saved_searches {
                    self.saved_searches_state
                        .select(Some(searches.len().saturating_sub(1)));
                }
            }
            CurrentScreen::InternalLogs => {
                if let Some(logs) = &self.internal_logs {
                    self.internal_logs_state
                        .select(Some(logs.len().saturating_sub(1)));
                }
            }
            CurrentScreen::Apps => {
                if let Some(apps) = &self.apps {
                    self.apps_state.select(Some(apps.len().saturating_sub(1)));
                }
            }
            CurrentScreen::Users => {
                if let Some(users) = &self.users {
                    self.users_state.select(Some(users.len().saturating_sub(1)));
                }
            }
            CurrentScreen::Cluster => {
                if self.cluster_view_mode == crate::app::state::ClusterViewMode::Peers
                    && let Some(peers) = &self.cluster_peers
                {
                    self.cluster_peers_state
                        .select(Some(peers.len().saturating_sub(1)));
                }
            }
            _ => {}
        }
    }

    /// Get the currently selected cluster peer, if any.
    pub fn get_selected_cluster_peer(&self) -> Option<&splunk_client::models::ClusterPeer> {
        if self.cluster_view_mode != crate::app::state::ClusterViewMode::Peers {
            return None;
        }
        let peers = self.cluster_peers.as_ref()?;
        let selected = self.cluster_peers_state.selected()?;
        peers.get(selected)
    }

    /// Get the currently selected SHC member, if any.
    pub fn get_selected_shc_member(&self) -> Option<&splunk_client::models::ShcMember> {
        if self.shc_view_mode != crate::app::state::ShcViewMode::Members {
            return None;
        }
        let members = self.shc_members.as_ref()?;
        let selected = self.shc_members_state.selected()?;
        members.get(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crate::app::state::ClusterViewMode;
    use splunk_client::models::ClusterPeer;

    fn create_mock_peers(count: usize) -> Vec<ClusterPeer> {
        (0..count)
            .map(|i| ClusterPeer {
                id: format!("peer-{}", i),
                label: Some(format!("Peer {}", i)),
                status: "Up".to_string(),
                peer_state: "Active".to_string(),
                site: Some("site1".to_string()),
                guid: format!("guid-{}", i),
                host: format!("host-{}", i),
                port: 8080 + i as u32,
                replication_count: Some(i as u32),
                replication_status: Some("Complete".to_string()),
                bundle_replication_count: None,
                is_captain: Some(i == 0),
            })
            .collect()
    }

    #[test]
    fn test_cluster_peers_next_item() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Peers;
        app.cluster_peers = Some(create_mock_peers(3));
        app.cluster_peers_state.select(Some(0));

        app.next_item();
        assert_eq!(app.cluster_peers_state.selected(), Some(1));

        app.next_item();
        assert_eq!(app.cluster_peers_state.selected(), Some(2));

        // Should not go past the end
        app.next_item();
        assert_eq!(app.cluster_peers_state.selected(), Some(2));
    }

    #[test]
    fn test_cluster_peers_previous_item() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Peers;
        app.cluster_peers = Some(create_mock_peers(3));
        app.cluster_peers_state.select(Some(2));

        app.previous_item();
        assert_eq!(app.cluster_peers_state.selected(), Some(1));

        app.previous_item();
        assert_eq!(app.cluster_peers_state.selected(), Some(0));

        // Should not go below 0
        app.previous_item();
        assert_eq!(app.cluster_peers_state.selected(), Some(0));
    }

    #[test]
    fn test_cluster_peers_navigation_in_summary_mode() {
        // Navigation should not work in Summary mode
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Summary;
        app.cluster_peers = Some(create_mock_peers(3));
        app.cluster_peers_state.select(Some(0));

        app.next_item();
        // Selection should remain unchanged
        assert_eq!(app.cluster_peers_state.selected(), Some(0));

        app.previous_item();
        assert_eq!(app.cluster_peers_state.selected(), Some(0));
    }

    #[test]
    fn test_cluster_peers_go_to_top() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Peers;
        app.cluster_peers = Some(create_mock_peers(5));
        app.cluster_peers_state.select(Some(4));

        app.go_to_top();
        assert_eq!(app.cluster_peers_state.selected(), Some(0));
    }

    #[test]
    fn test_cluster_peers_go_to_bottom() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Peers;
        app.cluster_peers = Some(create_mock_peers(5));
        app.cluster_peers_state.select(Some(0));

        app.go_to_bottom();
        assert_eq!(app.cluster_peers_state.selected(), Some(4));
    }

    #[test]
    fn test_cluster_peers_next_page() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Peers;
        app.cluster_peers = Some(create_mock_peers(15));
        app.cluster_peers_state.select(Some(0));

        app.next_page();
        assert_eq!(app.cluster_peers_state.selected(), Some(10));

        // Should clamp at the end
        app.next_page();
        assert_eq!(app.cluster_peers_state.selected(), Some(14));
    }

    #[test]
    fn test_cluster_peers_previous_page() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Peers;
        app.cluster_peers = Some(create_mock_peers(15));
        app.cluster_peers_state.select(Some(12));

        app.previous_page();
        assert_eq!(app.cluster_peers_state.selected(), Some(2));

        // Should not go below 0
        app.previous_page();
        assert_eq!(app.cluster_peers_state.selected(), Some(0));
    }

    #[test]
    fn test_cluster_peers_navigation_no_peers() {
        // Should not panic when peers is None
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Peers;
        app.cluster_peers = None;

        app.next_item();
        app.previous_item();
        app.go_to_top();
        app.go_to_bottom();
        app.next_page();
        app.previous_page();
        // Test passes if no panic occurs
    }

    #[test]
    fn test_cluster_peers_navigation_empty_peers() {
        // Should not panic when peers is empty
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Peers;
        app.cluster_peers = Some(vec![]);

        app.next_item();
        app.previous_item();
        app.go_to_top();
        app.go_to_bottom();
        app.next_page();
        app.previous_page();
        // Test passes if no panic occurs
    }
}
