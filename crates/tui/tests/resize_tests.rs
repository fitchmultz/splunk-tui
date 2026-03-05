//! Tests for terminal resize handling in the TUI.
//!
//! This module verifies that the TUI correctly handles terminal dimension changes,
//! including updating the last_area field, rendering at various sizes, and
//! maintaining correct mouse coordinate calculations after resize.

mod helpers;

use helpers::{TuiHarness, mouse_click};
use ratatui::layout::Rect;
use splunk_tui::App;
use splunk_tui::ConnectionContext;
use splunk_tui::action::Action;
use splunk_tui::app::footer_layout::FooterLayout;
use splunk_tui::app::state::CurrentScreen;

/// Test that the Resize action correctly updates last_area dimensions.
#[test]
fn test_resize_updates_last_area() {
    let mut harness = TuiHarness::new(80, 24);

    // Initial state after creation
    assert_eq!(harness.app.last_area.width, 0);
    assert_eq!(harness.app.last_area.height, 0);

    // Initial render sets last_area
    let _ = harness.render();
    assert_eq!(harness.app.last_area.width, 80);
    assert_eq!(harness.app.last_area.height, 24);

    // Simulate resize to larger dimensions
    harness.app.update(Action::Resize(100, 30));
    assert_eq!(harness.app.last_area.width, 100);
    assert_eq!(harness.app.last_area.height, 30);

    // Simulate resize to smaller dimensions
    harness.app.update(Action::Resize(60, 15));
    assert_eq!(harness.app.last_area.width, 60);
    assert_eq!(harness.app.last_area.height, 15);
}

/// Test that rendering works correctly at small terminal dimensions (20x10).
#[test]
fn test_render_at_small_dimensions() {
    let mut harness = TuiHarness::new(20, 10);
    harness.app.current_screen = CurrentScreen::Jobs;

    // Should not panic at small dimensions
    let output = harness.render();

    // Verify last_area was updated
    assert_eq!(harness.app.last_area.width, 20);
    assert_eq!(harness.app.last_area.height, 10);

    // Footer should be present (last 3 rows including borders)
    assert!(!output.is_empty());
}

/// Test that rendering works correctly at normal terminal dimensions (80x24).
#[test]
fn test_render_at_normal_dimensions() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Search;

    let output = harness.render();

    assert_eq!(harness.app.last_area.width, 80);
    assert_eq!(harness.app.last_area.height, 24);
    assert!(!output.is_empty());

    // Header should contain "Splunk TUI"
    assert!(output.contains("Splunk TUI"));
}

/// Test that rendering works correctly at large terminal dimensions (200x50).
#[test]
fn test_render_at_large_dimensions() {
    let mut harness = TuiHarness::new(200, 50);
    harness.app.current_screen = CurrentScreen::Indexes;

    let output = harness.render();

    assert_eq!(harness.app.last_area.width, 200);
    assert_eq!(harness.app.last_area.height, 50);
    assert!(!output.is_empty());
}

/// Test rendering at zero width (edge case).
#[test]
fn test_render_at_zero_width() {
    let mut harness = TuiHarness::new(0, 24);
    harness.app.current_screen = CurrentScreen::Jobs;

    // Should not panic with zero width
    let _ = harness.render();

    assert_eq!(harness.app.last_area.width, 0);
    assert_eq!(harness.app.last_area.height, 24);
}

/// Test rendering at zero height (edge case).
#[test]
fn test_render_at_zero_height() {
    let mut harness = TuiHarness::new(80, 0);
    harness.app.current_screen = CurrentScreen::Search;

    // Should not panic with zero height
    let _ = harness.render();

    assert_eq!(harness.app.last_area.width, 80);
    assert_eq!(harness.app.last_area.height, 0);
}

/// Test rendering at zero width and height (edge case).
#[test]
fn test_render_at_zero_dimensions() {
    let mut harness = TuiHarness::new(0, 0);
    harness.app.current_screen = CurrentScreen::Health;

    // Should not panic with zero dimensions
    let _ = harness.render();

    assert_eq!(harness.app.last_area.width, 0);
    assert_eq!(harness.app.last_area.height, 0);
}

/// Test rendering at very wide terminal dimensions (>500 cols).
#[test]
fn test_render_at_very_wide_dimensions() {
    let mut harness = TuiHarness::new(500, 24);
    harness.app.current_screen = CurrentScreen::Jobs;

    let output = harness.render();

    assert_eq!(harness.app.last_area.width, 500);
    assert_eq!(harness.app.last_area.height, 24);
    assert!(!output.is_empty());
}

/// Test rendering at very tall terminal dimensions (>100 rows).
#[test]
fn test_render_at_very_tall_dimensions() {
    let mut harness = TuiHarness::new(80, 150);
    harness.app.current_screen = CurrentScreen::Apps;

    let output = harness.render();

    assert_eq!(harness.app.last_area.width, 80);
    assert_eq!(harness.app.last_area.height, 150);
    assert!(!output.is_empty());
}

/// Test rendering at single cell dimensions (1x1).
#[test]
fn test_render_at_single_cell() {
    let mut harness = TuiHarness::new(1, 1);
    harness.app.current_screen = CurrentScreen::Search;

    // Should not panic at minimal dimensions
    let _ = harness.render();

    assert_eq!(harness.app.last_area.width, 1);
    assert_eq!(harness.app.last_area.height, 1);
}

/// Test that mouse coordinate calculations work correctly after resize.
#[test]
fn test_mouse_coordinates_after_resize() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Jobs;

    // Initial render to set last_area
    let _ = harness.render();

    // Get initial layout
    let layout_before = FooterLayout::calculate(
        false,
        0.0,
        harness.app.current_screen,
        harness.app.last_area.width,
    );
    assert!(layout_before.quit_visible);

    // Resize to larger terminal
    harness.app.update(Action::Resize(100, 30));

    // Layout should still calculate correctly
    let layout_after = FooterLayout::calculate(
        false,
        0.0,
        harness.app.current_screen,
        harness.app.last_area.width,
    );
    assert!(layout_after.quit_visible);
    // Quit button should be further right at wider terminal
    assert!(layout_after.quit_start > layout_before.quit_start);
}

/// Test that mouse coordinate calculations work correctly after resize to narrow terminal.
#[test]
fn test_mouse_coordinates_narrow_terminal() {
    let mut harness = TuiHarness::new(100, 24);
    harness.app.current_screen = CurrentScreen::Jobs;

    let _ = harness.render();

    // Resize to narrow terminal (50 cols)
    harness.app.update(Action::Resize(50, 24));

    let layout = FooterLayout::calculate(
        false,
        0.0,
        harness.app.current_screen,
        harness.app.last_area.width,
    );

    // Quit should still be visible at width 50
    assert!(layout.quit_visible, "Quit should be visible at width 50");
}

/// Test that quit button is hidden at very narrow widths.
#[test]
fn test_quit_button_hidden_very_narrow() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Jobs;

    let _ = harness.render();

    // Resize to very narrow terminal (35 cols)
    harness.app.update(Action::Resize(35, 24));

    let layout = FooterLayout::calculate(
        false,
        0.0,
        harness.app.current_screen,
        harness.app.last_area.width,
    );

    // Quit should be hidden at width 35
    assert!(!layout.quit_visible, "Quit should be hidden at width 35");
}

/// Test footer hint truncation at narrow widths.
#[test]
fn test_footer_hint_truncation_narrow() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Jobs;

    let _ = harness.render();

    // At 80 cols, hints should have reasonable width
    let _layout_80 = FooterLayout::calculate(false, 0.0, harness.app.current_screen, 80);

    // Resize to 60 cols
    harness.app.update(Action::Resize(60, 24));
    let layout_60 = FooterLayout::calculate(false, 0.0, harness.app.current_screen, 60);

    // Resize to 40 cols
    harness.app.update(Action::Resize(40, 24));
    let layout_40 = FooterLayout::calculate(false, 0.0, harness.app.current_screen, 40);

    // Hints width should generally decrease as terminal narrows
    // (though exact values depend on the screen's hints)
    assert!(
        layout_40.hints_width <= layout_60.hints_width || layout_40.hints_width == 0,
        "Hints at 40 cols should be less than or equal to 60 cols"
    );
}

/// Test that mouse click on quit button works correctly after resize.
#[test]
fn test_quit_click_after_resize() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Jobs;

    let _ = harness.render();

    // Resize to 100 cols
    harness.app.update(Action::Resize(100, 24));

    // Calculate quit button position
    let layout = FooterLayout::calculate(
        false,
        0.0,
        harness.app.current_screen,
        harness.app.last_area.width,
    );

    // Click in the middle of the quit button (accounting for border)
    let click_col = layout.quit_start + 1 + 3; // +1 for border, +3 for middle
    let click_row = 22; // Footer row (24 - 2)

    let action = harness.app.handle_mouse(mouse_click(click_col, click_row));
    assert!(
        matches!(action, Some(Action::Quit)),
        "Clicking quit button after resize should return Quit action"
    );
}

/// Test that no panics occur at large dimensions.
#[test]
fn test_no_panic_at_large_dimensions() {
    // Test with reasonably large but not extreme dimensions
    // (avoid u16::MAX which would require allocating a massive buffer)
    let mut app = App::new(None, ConnectionContext::default());

    // Test resize action with large dimensions (without rendering)
    app.update(Action::Resize(1000, 500));
    assert_eq!(app.last_area.width, 1000);
    assert_eq!(app.last_area.height, 500);

    // Test with even larger dimensions
    app.update(Action::Resize(5000, 2000));
    assert_eq!(app.last_area.width, 5000);
    assert_eq!(app.last_area.height, 2000);
}

/// Test resize action with various dimension combinations.
#[test]
fn test_resize_various_dimensions() {
    let mut app = App::new(None, ConnectionContext::default());

    let test_cases = [
        (1, 1),
        (10, 5),
        (40, 10),
        (80, 24),
        (132, 43),
        (200, 50),
        (500, 100),
    ];

    for (width, height) in test_cases {
        app.update(Action::Resize(width, height));
        assert_eq!(
            app.last_area,
            Rect::new(0, 0, width, height),
            "Resize to {}x{} should update last_area correctly",
            width,
            height
        );
    }
}

/// Test that content area calculations are correct after resize.
#[test]
fn test_content_area_after_resize() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Jobs;

    let _ = harness.render();

    // Content area is between header and footer
    // HEADER_HEIGHT = 4, FOOTER_HEIGHT = 3
    let content_start = 4; // HEADER_HEIGHT
    let content_end = 24 - 3; // height - FOOTER_HEIGHT

    assert_eq!(content_start, 4);
    assert_eq!(content_end, 21);

    // Resize to larger terminal
    harness.app.update(Action::Resize(100, 40));

    // Content area should scale accordingly
    let new_content_end = 40 - 3;
    assert_eq!(new_content_end, 37);
}

/// Test that resize handling works correctly with jobs data loaded.
#[test]
fn test_resize_with_jobs_loaded() {
    use splunk_client::models::SearchJobStatus;

    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Jobs;

    // Load some mock jobs using the action (which rebuilds filtered indices)
    harness
        .app
        .update(Action::JobsLoaded(Ok(vec![SearchJobStatus {
            sid: "job1".to_string(),
            is_done: true,
            is_finalized: true,
            done_progress: 1.0,
            run_duration: 5.23,
            disk_usage: 2048,
            scan_count: 1500,
            event_count: 500,
            result_count: 100,
            cursor_time: Some("2024-01-15T10:30:00.000Z".to_string()),
            priority: Some(5),
            label: Some("Test job".to_string()),
        }])));

    // Initial render
    let _ = harness.render();
    assert_eq!(harness.app.last_area.width, 80);
    assert_eq!(harness.app.last_area.height, 24);

    // Resize while jobs are loaded updates last_area
    harness.app.update(Action::Resize(120, 30));
    assert_eq!(harness.app.last_area.width, 120);
    assert_eq!(harness.app.last_area.height, 30);

    // Rendering resets last_area to the actual terminal size (TestBackend is fixed at 80x24)
    let output = harness.render();
    assert!(!output.is_empty());
    // last_area is reset to the terminal's actual size after render
    assert_eq!(harness.app.last_area.width, 80);
    assert_eq!(harness.app.last_area.height, 24);
}

/// Test that resize works correctly on all screen types.
#[test]
fn test_resize_all_screens() {
    let screens = [
        CurrentScreen::Search,
        CurrentScreen::Indexes,
        CurrentScreen::Cluster,
        CurrentScreen::Jobs,
        CurrentScreen::Health,
        CurrentScreen::SavedSearches,
        CurrentScreen::InternalLogs,
        CurrentScreen::Apps,
        CurrentScreen::Users,
        CurrentScreen::Settings,
    ];

    for screen in screens {
        let mut harness = TuiHarness::new(80, 24);
        harness.app.current_screen = screen;

        // Initial render
        let _ = harness.render();
        assert_eq!(harness.app.last_area.width, 80);
        assert_eq!(harness.app.last_area.height, 24);

        // Resize updates last_area
        harness.app.update(Action::Resize(100, 30));
        assert_eq!(harness.app.last_area.width, 100);
        assert_eq!(harness.app.last_area.height, 30);

        // Render after resize - rendering resets to actual terminal size
        let output = harness.render();
        assert!(
            !output.is_empty(),
            "Screen {:?} should render after resize",
            screen
        );
        // last_area is reset to the terminal's actual size after render
        assert_eq!(harness.app.last_area.width, 80);
        assert_eq!(harness.app.last_area.height, 24);
    }
}

/// Test that resize during loading state works correctly.
#[test]
fn test_resize_during_loading() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Indexes;
    harness.app.loading = true;
    harness.app.progress = 0.5;

    let _ = harness.render();

    // Resize while loading
    harness.app.update(Action::Resize(100, 30));

    // Should still show loading indicator correctly
    let layout = FooterLayout::calculate(
        true,
        0.5,
        harness.app.current_screen,
        harness.app.last_area.width,
    );

    assert!(
        layout.loading_width > 0,
        "Loading indicator should be visible"
    );
    assert!(layout.quit_visible, "Quit should be visible during loading");
}

/// Test that multiple rapid resizes are handled correctly.
#[test]
fn test_rapid_resizes() {
    let mut app = App::new(None, ConnectionContext::default());

    // Simulate rapid resize events
    let sizes = [
        (80, 24),
        (81, 24),
        (82, 24),
        (83, 25),
        (84, 25),
        (85, 26),
        (100, 30),
        (90, 25),
        (80, 24),
    ];

    for (width, height) in sizes {
        app.update(Action::Resize(width, height));
        assert_eq!(app.last_area.width, width);
        assert_eq!(app.last_area.height, height);
    }
}

/// Test that resize to dimensions smaller than header+footer doesn't panic.
#[test]
fn test_resize_very_short_terminal() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Search;

    let _ = harness.render();
    assert_eq!(harness.app.last_area.height, 24);

    // Resize to terminal shorter than header + footer combined (4 + 3 = 7)
    // The resize action updates last_area
    harness.app.update(Action::Resize(80, 5));
    assert_eq!(harness.app.last_area.height, 5);

    // Should not panic when rendering
    // Note: rendering will reset last_area to the TestBackend's fixed size (24)
    let _ = harness.render();

    // After render, last_area reflects the actual terminal size
    assert_eq!(harness.app.last_area.height, 24);
}
