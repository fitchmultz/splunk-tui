//! Progressive onboarding checklist state and milestone tracking.
//!
//! Responsibilities:
//! - Define onboarding milestones as bitflags for efficient storage
//! - Track completion state and session-aware auto-hide logic
//! - Rate-limit contextual hint toasts to prevent hint fatigue
//!
//! Does NOT handle:
//! - UI rendering (handled by widgets module)
//! - Persistence directly (delegated to config crate)

use std::collections::HashSet;
use std::time::Instant;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct OnboardingMilestones: u8 {
        const NONE = 0;
        const PROFILE_READY = 1 << 0;
        const CONNECTION_VERIFIED = 1 << 1;
        const FIRST_SEARCH_RUN = 1 << 2;
        const NAVIGATION_CYCLE_COMPLETED = 1 << 3;
        const HELP_OPENED = 1 << 4;
        const ALL = Self::PROFILE_READY.bits()
                   | Self::CONNECTION_VERIFIED.bits()
                   | Self::FIRST_SEARCH_RUN.bits()
                   | Self::NAVIGATION_CYCLE_COMPLETED.bits()
                   | Self::HELP_OPENED.bits();
    }
}

impl OnboardingMilestones {
    pub fn count(self) -> u8 {
        self.bits().count_ones() as u8
    }

    pub const TOTAL: u8 = 5;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingMilestone {
    ProfileReady,
    ConnectionVerified,
    FirstSearchRun,
    NavigationCycleCompleted,
    HelpOpened,
}

impl OnboardingMilestone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ProfileReady => "profile_ready",
            Self::ConnectionVerified => "connection_verified",
            Self::FirstSearchRun => "first_search_run",
            Self::NavigationCycleCompleted => "navigation_cycle_completed",
            Self::HelpOpened => "help_opened",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "profile_ready" => Some(Self::ProfileReady),
            "connection_verified" => Some(Self::ConnectionVerified),
            "first_search_run" => Some(Self::FirstSearchRun),
            "navigation_cycle_completed" => Some(Self::NavigationCycleCompleted),
            "help_opened" => Some(Self::HelpOpened),
            _ => None,
        }
    }

    pub fn to_flag(&self) -> OnboardingMilestones {
        match self {
            Self::ProfileReady => OnboardingMilestones::PROFILE_READY,
            Self::ConnectionVerified => OnboardingMilestones::CONNECTION_VERIFIED,
            Self::FirstSearchRun => OnboardingMilestones::FIRST_SEARCH_RUN,
            Self::NavigationCycleCompleted => OnboardingMilestones::NAVIGATION_CYCLE_COMPLETED,
            Self::HelpOpened => OnboardingMilestones::HELP_OPENED,
        }
    }

    pub fn hint_message(&self) -> &'static str {
        match self {
            Self::ProfileReady => "Create a profile to connect to Splunk",
            Self::ConnectionVerified => "Test your connection to verify setup",
            Self::FirstSearchRun => "Run a search query to explore your data",
            Self::NavigationCycleCompleted => "Press Tab to navigate between screens",
            Self::HelpOpened => "Press ? to see available keybindings",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            Self::ProfileReady => "Create Profile",
            Self::ConnectionVerified => "Test Connection",
            Self::FirstSearchRun => "Run Search",
            Self::NavigationCycleCompleted => "Navigate Screens",
            Self::HelpOpened => "View Help",
        }
    }

    pub const fn all() -> [Self; 5] {
        [
            Self::ProfileReady,
            Self::ConnectionVerified,
            Self::FirstSearchRun,
            Self::NavigationCycleCompleted,
            Self::HelpOpened,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct OnboardingChecklistState {
    pub milestones: OnboardingMilestones,
    pub dismissed_items: HashSet<String>,
    pub session_count: u8,
    pub sessions_since_completion: u8,
    pub globally_dismissed: bool,
    pub last_hint_at: Option<Instant>,
    pub hints_this_session: u8,
}

pub const AUTO_HIDE_SESSIONS: u8 = 3;
pub const HINT_COOLDOWN_SECS: u64 = 30;
pub const MAX_HINTS_PER_SESSION: u8 = 3;

impl Default for OnboardingChecklistState {
    fn default() -> Self {
        Self {
            milestones: OnboardingMilestones::NONE,
            dismissed_items: HashSet::new(),
            session_count: 0,
            sessions_since_completion: 0,
            globally_dismissed: false,
            last_hint_at: None,
            hints_this_session: 0,
        }
    }
}

impl OnboardingChecklistState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_milestone(&mut self, milestone: OnboardingMilestone) -> bool {
        let flag = milestone.to_flag();
        if self.milestones.contains(flag) {
            return false;
        }
        self.milestones |= flag;
        true
    }

    pub fn is_complete(&self) -> bool {
        self.milestones == OnboardingMilestones::ALL
    }

    pub fn progress(&self) -> (u8, u8) {
        (self.milestones.count(), OnboardingMilestones::TOTAL)
    }

    pub fn progress_percent(&self) -> u8 {
        let (completed, total) = self.progress();
        (completed as u16 * 100 / total as u16) as u8
    }

    pub fn should_show_checklist(&self) -> bool {
        if self.globally_dismissed {
            return false;
        }
        if self.is_complete() && self.sessions_since_completion >= AUTO_HIDE_SESSIONS {
            return false;
        }
        true
    }

    pub fn dismiss_item(&mut self, milestone: &OnboardingMilestone) {
        self.dismissed_items.insert(milestone.as_str().to_string());
    }

    pub fn is_item_dismissed(&self, milestone: &OnboardingMilestone) -> bool {
        self.dismissed_items.contains(milestone.as_str())
    }

    pub fn dismiss_all(&mut self) {
        self.globally_dismissed = true;
    }

    pub fn on_session_start(&mut self) {
        self.session_count = self.session_count.saturating_add(1);
        self.hints_this_session = 0;
        self.last_hint_at = None;
        if self.is_complete() {
            self.sessions_since_completion = self.sessions_since_completion.saturating_add(1);
        }
    }

    pub fn incomplete_milestones(&self) -> Vec<OnboardingMilestone> {
        OnboardingMilestone::all()
            .into_iter()
            .filter(|m| !self.milestones.contains(m.to_flag()) && !self.is_item_dismissed(m))
            .collect()
    }

    pub fn can_show_hint(&self) -> bool {
        if self.hints_this_session >= MAX_HINTS_PER_SESSION {
            return false;
        }
        if let Some(last) = self.last_hint_at {
            last.elapsed().as_secs() >= HINT_COOLDOWN_SECS
        } else {
            true
        }
    }

    pub fn record_hint_shown(&mut self) {
        self.last_hint_at = Some(Instant::now());
        self.hints_this_session = self.hints_this_session.saturating_add(1);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_milestone_bitflags() {
        let mut flags = OnboardingMilestones::NONE;
        assert!(!flags.contains(OnboardingMilestones::PROFILE_READY));

        flags |= OnboardingMilestones::PROFILE_READY;
        assert!(flags.contains(OnboardingMilestones::PROFILE_READY));
        assert_eq!(flags.count(), 1);

        flags |= OnboardingMilestones::CONNECTION_VERIFIED;
        assert_eq!(flags.count(), 2);
    }

    #[test]
    fn test_milestone_progress() {
        let mut state = OnboardingChecklistState::new();
        assert_eq!(state.progress(), (0, 5));
        assert_eq!(state.progress_percent(), 0);

        state.mark_milestone(OnboardingMilestone::ProfileReady);
        assert_eq!(state.progress(), (1, 5));

        state.mark_milestone(OnboardingMilestone::ConnectionVerified);
        state.mark_milestone(OnboardingMilestone::FirstSearchRun);
        state.mark_milestone(OnboardingMilestone::NavigationCycleCompleted);
        state.mark_milestone(OnboardingMilestone::HelpOpened);

        assert!(state.is_complete());
        assert_eq!(state.progress_percent(), 100);
    }

    #[test]
    fn test_auto_hide_after_sessions() {
        let mut state = OnboardingChecklistState::new();

        for m in OnboardingMilestone::all() {
            state.mark_milestone(m);
        }
        assert!(state.is_complete());
        assert!(state.should_show_checklist());

        state.on_session_start();
        assert_eq!(state.sessions_since_completion, 1);
        assert!(state.should_show_checklist());

        state.on_session_start();
        state.on_session_start();
        assert_eq!(state.sessions_since_completion, 3);
        assert!(!state.should_show_checklist());
    }

    #[test]
    fn test_dismiss_item() {
        let mut state = OnboardingChecklistState::new();
        let milestone = OnboardingMilestone::ProfileReady;

        assert!(!state.is_item_dismissed(&milestone));
        state.dismiss_item(&milestone);
        assert!(state.is_item_dismissed(&milestone));
    }

    #[test]
    fn test_rate_limiting() {
        let mut state = OnboardingChecklistState::new();

        assert!(state.can_show_hint());
        state.record_hint_shown();
        assert_eq!(state.hints_this_session, 1);

        state.record_hint_shown();
        state.record_hint_shown();
        assert_eq!(state.hints_this_session, 3);
        assert!(!state.can_show_hint());
    }
}
