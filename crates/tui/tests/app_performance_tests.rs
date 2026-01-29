//! Tests for performance benchmarks.
//!
//! This module tests:
//! - Search rendering performance with large datasets
//!
//! ## Invariants
//! - Rendering must complete within expected time bounds
//!
//! ## Test Organization
//! Tests focus on rendering performance.

use ratatui::{Terminal, backend::TestBackend};
use splunk_tui::{CurrentScreen, app::App, app::ConnectionContext};
use std::time::Instant;

#[test]
fn test_search_rendering_with_large_dataset() {
    let dataset_sizes = [10, 100, 1000, 10000];

    for size in dataset_sizes {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Search;

        let results: Vec<serde_json::Value> = (0..size)
            .map(|i| {
                serde_json::json!({
                    "_time": format!("2024-01-15T10:30:{:02}.000Z", i % 60),
                    "level": "INFO",
                    "message": format!("Event number {}", i),
                })
            })
            .collect();

        app.set_search_results(results);
        app.search_scroll_offset = 0;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

        let start = Instant::now();
        terminal.draw(|f| app.render(f)).expect("Failed to render");
        let duration = start.elapsed();

        let max_expected_ms = 10;
        assert!(
            duration.as_millis() < max_expected_ms,
            "Rendering {} results took {:?}, expected < {:?}ms",
            size,
            duration,
            max_expected_ms
        );
    }
}
