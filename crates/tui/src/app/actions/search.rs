//! Search action handlers for the TUI app.
//!
//! Responsibilities:
//! - Handle search lifecycle actions (SearchStarted, SearchComplete)
//! - Handle pagination of search results (MoreSearchResultsLoaded)
//! - Update search state and metadata

use crate::action::Action;
use crate::app::App;
use crate::ui::Toast;
use serde_json::Value;

impl App {
    /// Handle search-related actions.
    pub fn handle_search_action(&mut self, action: Action) {
        match action {
            Action::SearchStarted(query) => {
                self.running_query = Some(query);
            }
            Action::SearchComplete(Ok((results, sid, total))) => {
                self.handle_search_complete(results, sid, total);
            }
            Action::SearchComplete(Err((error_msg, details))) => {
                self.handle_search_error(error_msg, details);
            }
            Action::MoreSearchResultsLoaded(Ok((results, _offset, total))) => {
                self.append_search_results(results, total);
                self.loading = false;
            }
            Action::MoreSearchResultsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more results: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            _ => {}
        }
    }

    fn handle_search_complete(&mut self, results: Vec<Value>, sid: String, total: Option<u64>) {
        let results_count = results.len() as u64;
        self.set_search_results(results);
        self.search_sid = Some(sid);

        // Set pagination state from initial search results
        self.search_results_total_count = total;
        self.search_has_more_results = if let Some(t) = total {
            results_count < t
        } else {
            // When total is None, infer from page fullness
            // Note: initial fetch in main.rs uses 1000, but we use app's page_size for consistency
            results_count >= self.search_results_page_size
        };

        // Use running_query for status message, falling back to search_input if not set
        let query_for_status = self
            .running_query
            .take()
            .unwrap_or_else(|| self.search_input.clone());
        self.search_status = format!("Search complete: {}", query_for_status);
        self.loading = false;
    }

    fn handle_search_error(
        &mut self,
        error_msg: String,
        details: crate::error_details::ErrorDetails,
    ) {
        self.current_error = Some(details);
        self.toasts.push(Toast::error(error_msg));
        self.running_query = None; // Clear the running query on error
        self.loading = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;

    #[test]
    fn test_search_started_sets_running_query() {
        let mut app = App::new(None, ConnectionContext::default());
        let query = "index=main | stats count".to_string();

        app.handle_search_action(Action::SearchStarted(query.clone()));

        assert_eq!(app.running_query, Some(query));
    }

    #[test]
    fn test_search_complete_updates_state() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input = "test query".to_string();

        let results = vec![serde_json::json!({"_raw": "test event"})];
        let sid = "search_123".to_string();

        app.handle_search_action(Action::SearchComplete(Ok((
            results,
            sid.clone(),
            Some(100),
        ))));

        assert_eq!(app.search_sid, Some(sid));
        assert_eq!(app.search_results.len(), 1);
        assert_eq!(app.search_results_total_count, Some(100));
        assert!(!app.loading);
    }

    #[test]
    fn test_search_complete_with_total_none() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input = "test query".to_string();
        app.search_results_page_size = 10;

        // Results matching page size should indicate more results available
        let results: Vec<Value> = (0..10)
            .map(|i| serde_json::json!({"_raw": format!("event {}", i)}))
            .collect();

        app.handle_search_action(Action::SearchComplete(Ok((
            results,
            "sid".to_string(),
            None,
        ))));

        // When total is None and results count equals page size, has_more should be true
        assert!(app.search_has_more_results);
    }

    #[test]
    fn test_search_complete_clears_running_query() {
        let mut app = App::new(None, ConnectionContext::default());
        app.running_query = Some("previous query".to_string());
        app.search_input = "test query".to_string();

        let results: Vec<Value> = vec![];
        app.handle_search_action(Action::SearchComplete(Ok((
            results,
            "sid".to_string(),
            Some(0),
        ))));

        // running_query should be consumed and cleared
        assert!(app.running_query.is_none());
    }

    #[test]
    fn test_search_error_clears_running_query() {
        let mut app = App::new(None, ConnectionContext::default());
        app.running_query = Some("test query".to_string());

        let error_msg = "Search failed".to_string();
        let details = crate::error_details::ErrorDetails::from_error_string("Search failed");

        app.handle_search_action(Action::SearchComplete(Err((error_msg, details))));

        assert!(app.running_query.is_none());
        assert!(!app.loading);
        assert_eq!(app.toasts.len(), 1);
    }
}
