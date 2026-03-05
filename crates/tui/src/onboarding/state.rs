//! Tutorial state machine and progression logic.
//!
//! Responsibilities:
//! - Define tutorial step variants and their ordering
//! - Track tutorial progress and completion state
//! - Store intermediate data during the tutorial flow
//!
//! Does NOT handle:
//! - UI rendering (handled by the UI layer)
//! - Content strings (handled by the `steps` module)

/// Total number of tutorial steps (excluding Complete).
pub const TOTAL_STEPS: usize = 6;

/// Individual steps in the onboarding tutorial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TutorialStep {
    /// Welcome message and introduction to Splunk TUI.
    #[default]
    Welcome,
    /// Create a connection profile for Splunk.
    ProfileCreation,
    /// Test the connection to the Splunk server.
    ConnectionTest,
    /// Run the first search query.
    FirstSearch,
    /// Learn about keyboard shortcuts and navigation.
    KeybindingTutorial,
    /// Demonstrate exporting search results.
    ExportDemo,
    /// Tutorial completion screen.
    Complete,
}

impl TutorialStep {
    /// Returns the next step in the tutorial sequence.
    ///
    /// Returns `None` when called on `Complete`.
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Welcome => Some(Self::ProfileCreation),
            Self::ProfileCreation => Some(Self::ConnectionTest),
            Self::ConnectionTest => Some(Self::FirstSearch),
            Self::FirstSearch => Some(Self::KeybindingTutorial),
            Self::KeybindingTutorial => Some(Self::ExportDemo),
            Self::ExportDemo => Some(Self::Complete),
            Self::Complete => None,
        }
    }

    /// Returns the previous step in the tutorial sequence.
    ///
    /// Returns `None` when called on `Welcome`.
    pub fn previous(self) -> Option<Self> {
        match self {
            Self::Welcome => None,
            Self::ProfileCreation => Some(Self::Welcome),
            Self::ConnectionTest => Some(Self::ProfileCreation),
            Self::FirstSearch => Some(Self::ConnectionTest),
            Self::KeybindingTutorial => Some(Self::FirstSearch),
            Self::ExportDemo => Some(Self::KeybindingTutorial),
            Self::Complete => Some(Self::ExportDemo),
        }
    }

    /// Returns the display title for this step.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Welcome => "Welcome to Splunk TUI",
            Self::ProfileCreation => "Create a Connection Profile",
            Self::ConnectionTest => "Test Your Connection",
            Self::FirstSearch => "Run Your First Search",
            Self::KeybindingTutorial => "Learn the Keybindings",
            Self::ExportDemo => "Export Your Results",
            Self::Complete => "You're All Set!",
        }
    }

    /// Returns the step number (1-indexed) for progress display.
    /// Returns `None` for `Complete` as it is not a numbered step.
    pub fn step_number(&self) -> Option<usize> {
        match self {
            Self::Welcome => Some(1),
            Self::ProfileCreation => Some(2),
            Self::ConnectionTest => Some(3),
            Self::FirstSearch => Some(4),
            Self::KeybindingTutorial => Some(5),
            Self::ExportDemo => Some(6),
            Self::Complete => None,
        }
    }
}

/// Tracks the state and progress of the tutorial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TutorialState {
    /// The current step in the tutorial.
    pub current_step: TutorialStep,
    /// Whether the user has completed the tutorial at least once.
    pub has_completed: bool,
    /// Profile name being created during the tutorial.
    pub pending_profile_name: Option<String>,
    /// Result of the connection test.
    pub connection_test_result: Option<bool>,
    /// Whether the user has run their first search.
    pub has_run_first_search: bool,
    /// Whether the user has completed the export demo.
    pub has_exported: bool,
    /// Scroll offset for the keybinding tutorial view.
    pub keybinding_scroll_offset: usize,
}

impl TutorialState {
    /// Creates a new tutorial state starting at the Welcome step.
    pub fn new() -> Self {
        Self {
            current_step: TutorialStep::Welcome,
            has_completed: false,
            pending_profile_name: None,
            connection_test_result: None,
            has_run_first_search: false,
            has_exported: false,
            keybinding_scroll_offset: 0,
        }
    }

    /// Advances to the next step in the tutorial.
    ///
    /// Returns `true` if the step was advanced, `false` if already at Complete.
    pub fn next_step(&mut self) -> bool {
        if let Some(next) = self.current_step.next() {
            self.current_step = next;
            true
        } else {
            false
        }
    }

    /// Goes back to the previous step in the tutorial.
    ///
    /// Returns `true` if the step was moved back, `false` if already at Welcome.
    pub fn previous_step(&mut self) -> bool {
        if let Some(prev) = self.current_step.previous() {
            self.current_step = prev;
            true
        } else {
            false
        }
    }

    /// Marks the tutorial as completed.
    ///
    /// Sets `has_completed` to `true` and moves to the Complete step.
    pub fn complete(&mut self) {
        self.has_completed = true;
        self.current_step = TutorialStep::Complete;
    }

    /// Resets the tutorial to the beginning.
    ///
    /// Clears all progress but preserves `has_completed` status.
    pub fn reset(&mut self) {
        self.current_step = TutorialStep::Welcome;
        self.pending_profile_name = None;
        self.connection_test_result = None;
        self.has_run_first_search = false;
        self.has_exported = false;
        self.keybinding_scroll_offset = 0;
    }

    /// Returns the completion percentage (0-100).
    ///
    /// Complete step returns 100%, otherwise based on step number.
    pub fn progress_percent(&self) -> u8 {
        match self.current_step {
            TutorialStep::Complete => 100,
            step => {
                if let Some(num) = step.step_number() {
                    ((num.saturating_sub(1)) * 100 / TOTAL_STEPS) as u8
                } else {
                    0
                }
            }
        }
    }

    /// Returns whether the tutorial is at the Complete step.
    pub fn is_complete(&self) -> bool {
        self.current_step == TutorialStep::Complete
    }

    /// Returns whether the tutorial is at the Welcome step.
    pub fn is_at_start(&self) -> bool {
        self.current_step == TutorialStep::Welcome
    }

    /// Sets the pending profile name.
    pub fn set_pending_profile_name(&mut self, name: impl Into<String>) {
        self.pending_profile_name = Some(name.into());
    }

    /// Records the connection test result.
    pub fn set_connection_test_result(&mut self, success: bool) {
        self.connection_test_result = Some(success);
    }

    /// Marks that the user has run their first search.
    pub fn mark_first_search_complete(&mut self) {
        self.has_run_first_search = true;
    }

    /// Marks that the user has completed the export demo.
    pub fn mark_export_complete(&mut self) {
        self.has_exported = true;
    }

    /// Scrolls the keybinding tutorial view down.
    pub fn scroll_keybindings_down(&mut self, amount: usize) {
        self.keybinding_scroll_offset += amount;
    }

    /// Scrolls the keybinding tutorial view up.
    pub fn scroll_keybindings_up(&mut self, amount: usize) {
        self.keybinding_scroll_offset = self.keybinding_scroll_offset.saturating_sub(amount);
    }
}

impl Default for TutorialState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tutorial_step_next() {
        assert_eq!(
            TutorialStep::Welcome.next(),
            Some(TutorialStep::ProfileCreation)
        );
        assert_eq!(
            TutorialStep::ProfileCreation.next(),
            Some(TutorialStep::ConnectionTest)
        );
        assert_eq!(
            TutorialStep::ConnectionTest.next(),
            Some(TutorialStep::FirstSearch)
        );
        assert_eq!(
            TutorialStep::FirstSearch.next(),
            Some(TutorialStep::KeybindingTutorial)
        );
        assert_eq!(
            TutorialStep::KeybindingTutorial.next(),
            Some(TutorialStep::ExportDemo)
        );
        assert_eq!(
            TutorialStep::ExportDemo.next(),
            Some(TutorialStep::Complete)
        );
        assert_eq!(TutorialStep::Complete.next(), None);
    }

    #[test]
    fn test_tutorial_step_previous() {
        assert_eq!(TutorialStep::Welcome.previous(), None);
        assert_eq!(
            TutorialStep::ProfileCreation.previous(),
            Some(TutorialStep::Welcome)
        );
        assert_eq!(
            TutorialStep::ConnectionTest.previous(),
            Some(TutorialStep::ProfileCreation)
        );
        assert_eq!(
            TutorialStep::FirstSearch.previous(),
            Some(TutorialStep::ConnectionTest)
        );
        assert_eq!(
            TutorialStep::KeybindingTutorial.previous(),
            Some(TutorialStep::FirstSearch)
        );
        assert_eq!(
            TutorialStep::ExportDemo.previous(),
            Some(TutorialStep::KeybindingTutorial)
        );
        assert_eq!(
            TutorialStep::Complete.previous(),
            Some(TutorialStep::ExportDemo)
        );
    }

    #[test]
    fn test_tutorial_step_titles() {
        assert!(TutorialStep::Welcome.title().contains("Welcome"));
        assert!(TutorialStep::ProfileCreation.title().contains("Profile"));
        assert!(TutorialStep::ConnectionTest.title().contains("Connection"));
        assert!(TutorialStep::FirstSearch.title().contains("Search"));
        assert!(
            TutorialStep::KeybindingTutorial
                .title()
                .contains("Keybinding")
        );
        assert!(TutorialStep::ExportDemo.title().contains("Export"));
        assert!(TutorialStep::Complete.title().contains("All Set"));
    }

    #[test]
    fn test_tutorial_step_step_numbers() {
        assert_eq!(TutorialStep::Welcome.step_number(), Some(1));
        assert_eq!(TutorialStep::ProfileCreation.step_number(), Some(2));
        assert_eq!(TutorialStep::ConnectionTest.step_number(), Some(3));
        assert_eq!(TutorialStep::FirstSearch.step_number(), Some(4));
        assert_eq!(TutorialStep::KeybindingTutorial.step_number(), Some(5));
        assert_eq!(TutorialStep::ExportDemo.step_number(), Some(6));
        assert_eq!(TutorialStep::Complete.step_number(), None);
    }

    #[test]
    fn test_tutorial_step_default() {
        let step: TutorialStep = Default::default();
        assert_eq!(step, TutorialStep::Welcome);
    }

    #[test]
    fn test_tutorial_state_new() {
        let state = TutorialState::new();
        assert_eq!(state.current_step, TutorialStep::Welcome);
        assert!(!state.has_completed);
        assert!(state.pending_profile_name.is_none());
        assert!(state.connection_test_result.is_none());
        assert!(!state.has_run_first_search);
        assert!(!state.has_exported);
        assert_eq!(state.keybinding_scroll_offset, 0);
    }

    #[test]
    fn test_tutorial_state_default() {
        let state: TutorialState = Default::default();
        assert_eq!(state.current_step, TutorialStep::Welcome);
        assert!(!state.has_completed);
    }

    #[test]
    fn test_tutorial_state_next_step() {
        let mut state = TutorialState::new();
        assert!(state.next_step());
        assert_eq!(state.current_step, TutorialStep::ProfileCreation);
        assert!(state.next_step());
        assert_eq!(state.current_step, TutorialStep::ConnectionTest);
    }

    #[test]
    fn test_tutorial_state_next_step_at_end() {
        let mut state = TutorialState::new();
        state.current_step = TutorialStep::Complete;
        assert!(!state.next_step());
        assert_eq!(state.current_step, TutorialStep::Complete);
    }

    #[test]
    fn test_tutorial_state_previous_step() {
        let mut state = TutorialState::new();
        state.current_step = TutorialStep::ProfileCreation;
        assert!(state.previous_step());
        assert_eq!(state.current_step, TutorialStep::Welcome);
    }

    #[test]
    fn test_tutorial_state_previous_step_at_start() {
        let mut state = TutorialState::new();
        assert!(!state.previous_step());
        assert_eq!(state.current_step, TutorialStep::Welcome);
    }

    #[test]
    fn test_tutorial_state_complete() {
        let mut state = TutorialState::new();
        state.complete();
        assert!(state.has_completed);
        assert_eq!(state.current_step, TutorialStep::Complete);
        assert!(state.is_complete());
    }

    #[test]
    fn test_tutorial_state_reset() {
        let mut state = TutorialState::new();
        state.complete();
        state.set_pending_profile_name("test-profile");
        state.set_connection_test_result(true);
        state.mark_first_search_complete();
        state.mark_export_complete();
        state.scroll_keybindings_down(5);

        state.reset();

        assert_eq!(state.current_step, TutorialStep::Welcome);
        assert!(state.has_completed); // Preserved
        assert!(state.pending_profile_name.is_none());
        assert!(state.connection_test_result.is_none());
        assert!(!state.has_run_first_search);
        assert!(!state.has_exported);
        assert_eq!(state.keybinding_scroll_offset, 0);
    }

    #[test]
    fn test_tutorial_state_progress_percent() {
        let mut state = TutorialState::new();
        assert_eq!(state.progress_percent(), 0);

        state.current_step = TutorialStep::ProfileCreation;
        assert_eq!(state.progress_percent(), 16); // 1/6

        state.current_step = TutorialStep::ConnectionTest;
        assert_eq!(state.progress_percent(), 33); // 2/6

        state.current_step = TutorialStep::FirstSearch;
        assert_eq!(state.progress_percent(), 50); // 3/6

        state.current_step = TutorialStep::KeybindingTutorial;
        assert_eq!(state.progress_percent(), 66); // 4/6

        state.current_step = TutorialStep::ExportDemo;
        assert_eq!(state.progress_percent(), 83); // 5/6

        state.current_step = TutorialStep::Complete;
        assert_eq!(state.progress_percent(), 100);
    }

    #[test]
    fn test_tutorial_state_is_complete() {
        let mut state = TutorialState::new();
        assert!(!state.is_complete());
        state.complete();
        assert!(state.is_complete());
    }

    #[test]
    fn test_tutorial_state_is_at_start() {
        let mut state = TutorialState::new();
        assert!(state.is_at_start());
        state.next_step();
        assert!(!state.is_at_start());
    }

    #[test]
    fn test_tutorial_state_setters() {
        let mut state = TutorialState::new();

        state.set_pending_profile_name("my-profile");
        assert_eq!(state.pending_profile_name, Some("my-profile".to_string()));

        state.set_connection_test_result(true);
        assert_eq!(state.connection_test_result, Some(true));

        state.mark_first_search_complete();
        assert!(state.has_run_first_search);

        state.mark_export_complete();
        assert!(state.has_exported);
    }

    #[test]
    fn test_tutorial_state_scroll_keybindings() {
        let mut state = TutorialState::new();

        state.scroll_keybindings_down(5);
        assert_eq!(state.keybinding_scroll_offset, 5);

        state.scroll_keybindings_down(3);
        assert_eq!(state.keybinding_scroll_offset, 8);

        state.scroll_keybindings_up(2);
        assert_eq!(state.keybinding_scroll_offset, 6);

        state.scroll_keybindings_up(10);
        assert_eq!(state.keybinding_scroll_offset, 0); // Saturated
    }

    #[test]
    fn test_total_steps_constant() {
        assert_eq!(TOTAL_STEPS, 6);
    }
}
