//! Tests for indexes screen navigation.
//!
//! This module tests:
//! - Up/down navigation in indexes list
//! - Selection state management
//!
//! ## Invariants
//! - Navigation must stay within bounds of the indexes list
//!
//! ## Test Organization
//! Tests focus on list navigation behavior.

use splunk_client::models::Index;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext};

#[test]
fn test_indexes_navigation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;
    app.indexes = Some(vec![
        Index {
            name: "index1".to_string(),
            total_event_count: 100,
            current_db_size_mb: 10,
            max_total_data_size_mb: None,
            max_warm_db_count: None,
            max_hot_buckets: None,
            frozen_time_period_in_secs: None,
            cold_db_path: None,
            home_path: None,
            thawed_path: None,
            cold_to_frozen_dir: None,
            primary_index: None,
        },
        Index {
            name: "index2".to_string(),
            total_event_count: 200,
            current_db_size_mb: 20,
            max_total_data_size_mb: None,
            max_warm_db_count: None,
            max_hot_buckets: None,
            frozen_time_period_in_secs: None,
            cold_db_path: None,
            home_path: None,
            thawed_path: None,
            cold_to_frozen_dir: None,
            primary_index: None,
        },
    ]);
    app.indexes_state.select(Some(0));

    // Navigate down
    app.update(Action::NavigateDown);
    assert_eq!(
        app.indexes_state.selected(),
        Some(1),
        "Should move to index 1"
    );

    // Navigate up
    app.update(Action::NavigateUp);
    assert_eq!(
        app.indexes_state.selected(),
        Some(0),
        "Should move to index 0"
    );
}
