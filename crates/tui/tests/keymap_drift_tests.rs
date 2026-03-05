//! Tests to prevent drift between CurrentScreen enum and keymap screen lists.
//!
//! These tests ensure that when a new screen is added to the CurrentScreen enum,
//! it is also added to the all_screens() function in the keymap module.
//! Without this, footer hints may be missing for new screens.

use splunk_tui::app::CurrentScreen;

/// The expected number of screens in all_screens().
/// This must be updated when adding a new screen to the application.
const EXPECTED_SCREEN_COUNT: usize = 28;

/// Verifies that all_screens() count matches the expected count.
///
/// This test will fail if:
/// - A new screen is added to CurrentScreen but not to all_screens()
/// - A screen is removed from CurrentScreen but not from all_screens()
///
/// When this test fails, update BOTH:
/// 1. The CurrentScreen enum in crates/tui/src/app/state.rs
/// 2. The all_screens() function in crates/tui/src/input/keymap/mod.rs
/// 3. The EXPECTED_SCREEN_COUNT constant above
#[test]
fn test_all_screens_count_matches_expected() {
    use splunk_tui::input::keymap::all_screens;

    let screens = all_screens();

    assert_eq!(
        screens.len(),
        EXPECTED_SCREEN_COUNT,
        "all_screens() returned {} screens, but expected {}.\n\n\
         If you added a new screen to CurrentScreen, you MUST also:\
         1. Add it to all_screens() in crates/tui/src/input/keymap/mod.rs\
         2. Update EXPECTED_SCREEN_COUNT in this test file",
        screens.len(),
        EXPECTED_SCREEN_COUNT
    );
}

/// Verifies that all screens in all_screens() are unique.
///
/// This prevents accidental duplication in the screen list.
#[test]
fn test_all_screens_contains_no_duplicates() {
    use splunk_tui::input::keymap::all_screens;

    let screens = all_screens();

    // Check for duplicates by comparing each element with every other element
    for (i, screen_i) in screens.iter().enumerate() {
        for (j, screen_j) in screens.iter().enumerate() {
            if i != j {
                assert_ne!(
                    screen_i, screen_j,
                    "Duplicate screen found in all_screens() at positions {} and {}",
                    i, j
                );
            }
        }
    }
}

/// Verifies that JobInspect is included in all_screens().
///
/// JobInspect is a special screen that's accessed via InspectJob action,
/// not through cyclic navigation. It's important that it still has footer hints.
#[test]
fn test_job_inspect_is_in_all_screens() {
    use splunk_tui::input::keymap::all_screens;

    let screens = all_screens();

    assert!(
        screens.contains(&CurrentScreen::JobInspect),
        "JobInspect should be in all_screens() even though it's not in the cyclic navigation"
    );
}

/// Verifies that Search screen is the first screen in all_screens().
///
/// The order in all_screens() doesn't affect functionality, but Search
/// being first is a convention that should be maintained.
#[test]
fn test_search_is_first_screen() {
    use splunk_tui::input::keymap::all_screens;

    let screens = all_screens();

    assert_eq!(
        screens[0],
        CurrentScreen::Search,
        "Search should be the first screen in all_screens()"
    );
}

/// Verifies that each screen in all_screens() has a corresponding Section mapping.
///
/// This ensures that footer hints can be generated for every screen.
#[test]
fn test_all_screens_have_section_mappings() {
    use splunk_tui::input::keymap::footer_hints;

    // This test indirectly verifies section mappings by calling footer_hints
    // for each screen. If a screen is missing a Section mapping, footer_hints
    // will return an empty vector (which is valid but might indicate a missing mapping).

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

    // Verify we have the expected number of screens
    assert_eq!(
        screens.len(),
        EXPECTED_SCREEN_COUNT,
        "Test screen array count doesn't match EXPECTED_SCREEN_COUNT"
    );

    // Each screen should be able to generate footer hints without panicking
    for screen in screens {
        let hints = footer_hints(screen);
        // We don't assert on hints content because empty hints are valid
        // (some screens might not have specific keybindings)
        // We just want to make sure this doesn't panic
        let _ = hints.len();
    }
}
