//! Tutorial content and help text.
//!
//! Responsibilities:
//! - Provide detailed help text for each tutorial step
//! - Generate footer hints showing available keybindings
//!
//! Does NOT handle:
//! - State management (handled by the `state` module)
//! - UI rendering (handled by the UI layer)

use super::state::TutorialStep;

/// Content provider for tutorial steps.
pub struct TutorialSteps;

impl TutorialSteps {
    /// Returns the detailed content for the given tutorial step.
    ///
    /// Content is formatted for display in the tutorial modal or screen.
    pub fn content(step: &TutorialStep) -> String {
        match step {
            TutorialStep::Welcome => Self::welcome_content(),
            TutorialStep::ProfileCreation => Self::profile_creation_content(),
            TutorialStep::ConnectionTest => Self::connection_test_content(),
            TutorialStep::FirstSearch => Self::first_search_content(),
            TutorialStep::KeybindingTutorial => Self::keybinding_tutorial_content(),
            TutorialStep::ExportDemo => Self::export_demo_content(),
            TutorialStep::Complete => Self::complete_content(),
        }
    }

    /// Returns a short hint for the footer showing available actions.
    ///
    /// These hints are context-sensitive based on the current step.
    pub fn footer_hint(step: &TutorialStep) -> String {
        match step {
            TutorialStep::Welcome => super::tutorial_keybindings::welcome_footer_hint(),
            TutorialStep::ProfileCreation => {
                "Press → to continue after creating profile | ← to go back".to_string()
            }
            TutorialStep::ConnectionTest => {
                "Press 't' to test connection | → to continue | ← to go back".to_string()
            }
            TutorialStep::FirstSearch => {
                "Press → to continue after running search | ← to go back".to_string()
            }
            TutorialStep::KeybindingTutorial => {
                "↑/↓ to scroll | → to continue | ← to go back".to_string()
            }
            TutorialStep::ExportDemo => {
                "Press → to continue after exporting | ← to go back".to_string()
            }
            TutorialStep::Complete => {
                "Press Enter to finish | Press r to restart tutorial".to_string()
            }
        }
    }

    fn welcome_content() -> String {
        r#"Welcome to Splunk TUI!

This interactive tutorial will guide you through the basics of using the Splunk Terminal User Interface.

In this tutorial, you will learn how to:
  • Create a connection profile to your Splunk server
  • Test your connection
  • Run your first search
  • Navigate using keyboard shortcuts
  • Export search results

Let's get started!

Tip: You can skip this tutorial at any time by pressing 'q'. You can always restart it from the Settings screen."#.to_string()
    }

    fn profile_creation_content() -> String {
        r#"Step 2: Create a Connection Profile

Before you can use Splunk TUI, you need to create a connection profile that stores your Splunk server connection details.

Press Enter to open the profile creation form, then:
  1. Enter a memorable profile name
  2. Enter your Splunk server URL (e.g., https://localhost:8089)
  3. Enter your username and password, or API token
  4. Press Enter to save

Your credentials are stored securely and are only used to connect to your Splunk server.

After saving, press → to continue."#.to_string()
    }

    fn connection_test_content() -> String {
        r#"Step 3: Test Your Connection

Now that you have created a profile, let's verify that we can connect to your Splunk server.

The connection test will:
  • Verify the server is reachable
  • Check your authentication credentials
  • Confirm TLS certificate status

To test the connection:
  1. Select your profile from the list
  2. Press 't' to run connection diagnostics
  3. Review the pass/fail status for each check

If the test fails, check:
  • Your Splunk server is running
  • The URL and port are correct
  • Your credentials are valid
  • TLS certificate verification settings (if using self-signed certs)

Once the connection test passes, press → to continue."#
            .to_string()
    }

    fn first_search_content() -> String {
        r#"Step 4: Run Your First Search

Now let's run a search to see Splunk TUI in action!

To run a search:
  1. Navigate to the Search screen (it's the first screen)
  2. Type a search query in the input box at the top
  3. Press Enter to execute the search

Try one of these example searches:
  • index=_internal | head 10
  • index=main | stats count by source
  • | makeresults count=5

The results will appear below the search box. You can:
  • Scroll through results with ↑/↓
  • Page up/down with PageUp/PageDown
  • Sort columns by clicking or pressing Tab

Run a search and then press → to continue."#
            .to_string()
    }

    fn keybinding_tutorial_content() -> String {
        super::tutorial_keybindings::generate_keybinding_section()
    }

    fn export_demo_content() -> String {
        r#"Step 5: Export Your Results

Splunk TUI allows you to export search results to various formats for use in other tools.

Supported export formats:
  • CSV  - Comma-separated values (great for Excel)
  • JSON - Structured JSON format (great for APIs)
  • Raw  - Raw events as plain text

To export results:
  1. Run a search and wait for results
  2. Press 'e' to open the export dialog
  3. Select your preferred format
  4. Choose the output file location
  5. Confirm to export

The export will include all currently loaded results. For large result sets, consider using the CLI export command instead.

Try exporting some results and then press → to continue."#.to_string()
    }

    fn complete_content() -> String {
        let screen_nav_keys = super::tutorial_keybindings::screen_navigation_keys_text();
        let help_key = super::tutorial_keybindings::help_key_text();

        format!(
            r#"You're All Set! 🎉

Congratulations! You've completed the Splunk TUI tutorial.

You now know how to:
  ✓ Create and manage connection profiles
  ✓ Test connections to your Splunk server
  ✓ Run searches and view results
  ✓ Navigate using keyboard shortcuts
  ✓ Export results to various formats

What's Next?
  • Explore the different screens with {screen_nav_keys}
  • Check out cluster health monitoring
  • Browse indexes and saved searches
  • View system jobs and their status
  • Customize your experience in Settings

Remember: Press '{help_key}' at any time to see available keybindings for the current screen.

Happy Splunking!"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_steps_have_content() {
        let steps = [
            TutorialStep::Welcome,
            TutorialStep::ProfileCreation,
            TutorialStep::ConnectionTest,
            TutorialStep::FirstSearch,
            TutorialStep::KeybindingTutorial,
            TutorialStep::ExportDemo,
            TutorialStep::Complete,
        ];

        for step in &steps {
            let content = TutorialSteps::content(step);
            assert!(!content.is_empty(), "Step {:?} should have content", step);
            // Content should be reasonably detailed (at least 50 characters)
            assert!(
                content.len() > 50,
                "Step {:?} content should be detailed",
                step
            );
        }
    }

    #[test]
    fn test_all_steps_have_footer_hint() {
        let steps = [
            TutorialStep::Welcome,
            TutorialStep::ProfileCreation,
            TutorialStep::ConnectionTest,
            TutorialStep::FirstSearch,
            TutorialStep::KeybindingTutorial,
            TutorialStep::ExportDemo,
            TutorialStep::Complete,
        ];

        for step in &steps {
            let hint = TutorialSteps::footer_hint(step);
            assert!(
                !hint.is_empty(),
                "Step {:?} should have a footer hint",
                step
            );
            // Hint should contain navigation guidance
            assert!(
                hint.contains('→') || hint.contains("Enter") || hint.contains("skip"),
                "Step {:?} hint should contain navigation guidance",
                step
            );
        }
    }

    #[test]
    fn test_welcome_content_structure() {
        let content = TutorialSteps::content(&TutorialStep::Welcome);
        assert!(content.contains("Welcome to Splunk TUI"));
        assert!(content.contains("tutorial"));
        assert!(content.contains("profile"));
        assert!(content.contains("search"));
    }

    #[test]
    fn test_profile_creation_content_structure() {
        let content = TutorialSteps::content(&TutorialStep::ProfileCreation);
        assert!(content.contains("Profile"));
        assert!(content.contains("URL"));
        assert!(content.contains("password") || content.contains("token"));
    }

    #[test]
    fn test_connection_test_content_structure() {
        let content = TutorialSteps::content(&TutorialStep::ConnectionTest);
        assert!(content.contains("connection"));
        assert!(content.contains("test"));
        assert!(content.contains("server"));
    }

    #[test]
    fn test_first_search_content_structure() {
        let content = TutorialSteps::content(&TutorialStep::FirstSearch);
        assert!(content.contains("search"));
        assert!(content.contains("index"));
        assert!(content.contains("Enter"));
    }

    #[test]
    fn test_keybinding_tutorial_content_structure() {
        let content = TutorialSteps::content(&TutorialStep::KeybindingTutorial);
        assert!(content.contains("keybinding") || content.contains("shortcut"));
        assert!(content.contains("Tab") || content.contains("→"));
    }

    #[test]
    fn test_export_demo_content_structure() {
        let content = TutorialSteps::content(&TutorialStep::ExportDemo);
        assert!(content.contains("export"));
        assert!(content.contains("CSV") || content.contains("JSON"));
    }

    #[test]
    fn test_complete_content_structure() {
        let content = TutorialSteps::content(&TutorialStep::Complete);
        assert!(content.contains("All Set") || content.contains("Congratulations"));
        assert!(content.contains('✓') || content.contains("completed"));
    }

    #[test]
    fn test_welcome_footer_hint() {
        let hint = TutorialSteps::footer_hint(&TutorialStep::Welcome);
        assert!(hint.contains('→') || hint.contains("Enter"));
        assert!(hint.contains("skip") || hint.contains('q'));
    }

    #[test]
    fn test_complete_footer_hint() {
        let hint = TutorialSteps::footer_hint(&TutorialStep::Complete);
        assert!(hint.contains("Enter") || hint.contains("finish"));
        assert!(hint.contains('r') || hint.contains("restart"));
    }

    #[test]
    fn test_footer_hints_consistency() {
        let numbered_steps = [
            TutorialStep::Welcome,
            TutorialStep::ProfileCreation,
            TutorialStep::ConnectionTest,
            TutorialStep::FirstSearch,
            TutorialStep::KeybindingTutorial,
            TutorialStep::ExportDemo,
        ];

        // All numbered steps should have navigation arrows in hints
        for step in &numbered_steps {
            let hint = TutorialSteps::footer_hint(step);
            assert!(
                hint.contains('→') || hint.contains("continue"),
                "Step {:?} hint should indicate how to continue",
                step
            );
        }
    }
}
