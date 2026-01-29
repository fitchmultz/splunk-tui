//! Snapshot tests for Search screen rendering.

mod helpers;

use helpers::TuiHarness;

#[test]
fn snapshot_search_screen_initial() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main".to_string();

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_loading() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main | stats count".to_string();
    harness.app.search_status = "Running search...".to_string();
    harness.app.loading = true;
    harness.app.progress = 0.45;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_with_results() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main ERROR".to_string();
    harness.app.search_status = "Search complete: index=main ERROR".to_string();
    harness.app.set_search_results(vec![
        serde_json::json!({"_time": "2024-01-15T10:30:00.000Z", "level": "ERROR", "message": "Connection failed"}),
        serde_json::json!({"_time": "2024-01-15T10:29:00.000Z", "level": "ERROR", "message": "Timeout error"}),
    ]);
    harness.app.search_sid = Some("search_12345".to_string());

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_empty() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input.clear();
    harness.app.set_search_results(Vec::new());

    insta::assert_snapshot!(harness.render());
}

// Cursor visibility tests (RQ-0110)

#[test]
fn snapshot_search_screen_cursor_at_end() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main".to_string();
    harness.app.search_cursor_position = 10; // At end
    harness.app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_cursor_in_middle() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main".to_string();
    harness.app.search_cursor_position = 5; // After "index"
    harness.app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_cursor_at_start() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main".to_string();
    harness.app.search_cursor_position = 0; // At start
    harness.app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_cursor_hidden_in_results_mode() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main".to_string();
    harness.app.search_cursor_position = 5;
    harness.app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_cursor_with_empty_input() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input.clear();
    harness.app.search_cursor_position = 0;
    harness.app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_virtual_window_scrolling() {
    {
        let mut harness = TuiHarness::new(80, 24);
        harness.app.current_screen = splunk_tui::CurrentScreen::Search;
        harness.app.search_input = "test query".to_string();

        let results: Vec<serde_json::Value> = (0..100)
            .map(|i| serde_json::json!({"id": i, "message": format!("Message {}", i)}))
            .collect();

        harness.app.set_search_results(results);
        harness.app.search_scroll_offset = 0;

        insta::assert_snapshot!(harness.render());
    }

    {
        let mut harness = TuiHarness::new(80, 24);
        harness.app.current_screen = splunk_tui::CurrentScreen::Search;
        harness.app.search_input = "test query".to_string();

        let results: Vec<serde_json::Value> = (0..100)
            .map(|i| serde_json::json!({"id": i, "message": format!("Message {}", i)}))
            .collect();

        harness.app.set_search_results(results);
        harness.app.search_scroll_offset = 50;

        insta::assert_snapshot!(harness.render());
    }

    {
        let mut harness = TuiHarness::new(80, 24);
        harness.app.current_screen = splunk_tui::CurrentScreen::Search;
        harness.app.search_input = "test query".to_string();

        let results: Vec<serde_json::Value> = (0..100)
            .map(|i| serde_json::json!({"id": i, "message": format!("Message {}", i)}))
            .collect();

        harness.app.set_search_results(results);
        harness.app.search_scroll_offset = 90;

        insta::assert_snapshot!(harness.render());
    }
}
