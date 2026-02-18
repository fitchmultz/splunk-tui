//! Tests for progressive onboarding checklist functionality.

use splunk_tui::onboarding::checklist::{
    AUTO_HIDE_SESSIONS, MAX_HINTS_PER_SESSION, OnboardingChecklistState, OnboardingMilestone,
    OnboardingMilestones,
};

#[test]
fn test_checklist_initial_state() {
    let state = OnboardingChecklistState::new();
    assert_eq!(state.milestones, OnboardingMilestones::NONE);
    assert!(!state.is_complete());
    assert_eq!(state.progress(), (0, 5));
}

#[test]
fn test_mark_milestone() {
    let mut state = OnboardingChecklistState::new();

    let changed = state.mark_milestone(OnboardingMilestone::ProfileReady);
    assert!(changed);
    assert!(
        state
            .milestones
            .contains(OnboardingMilestones::PROFILE_READY)
    );

    let changed_again = state.mark_milestone(OnboardingMilestone::ProfileReady);
    assert!(!changed_again);
}

#[test]
fn test_complete_all_milestones() {
    let mut state = OnboardingChecklistState::new();

    for milestone in OnboardingMilestone::all() {
        state.mark_milestone(milestone);
    }

    assert!(state.is_complete());
    assert_eq!(state.progress_percent(), 100);
}

#[test]
fn test_auto_hide_behavior() {
    let mut state = OnboardingChecklistState::new();

    assert!(state.should_show_checklist());

    for milestone in OnboardingMilestone::all() {
        state.mark_milestone(milestone);
    }

    assert!(state.should_show_checklist());

    for _ in 0..AUTO_HIDE_SESSIONS {
        state.on_session_start();
    }
    assert!(!state.should_show_checklist());
}

#[test]
fn test_global_dismiss() {
    let mut state = OnboardingChecklistState::new();
    assert!(state.should_show_checklist());

    state.dismiss_all();
    assert!(!state.should_show_checklist());
}

#[test]
fn test_item_dismissal() {
    let mut state = OnboardingChecklistState::new();
    let milestone = OnboardingMilestone::ProfileReady;

    let incomplete = state.incomplete_milestones();
    assert!(incomplete.contains(&milestone));

    state.dismiss_item(&milestone);
    let incomplete_after = state.incomplete_milestones();
    assert!(!incomplete_after.contains(&milestone));
}

#[test]
fn test_hint_rate_limiting() {
    let mut state = OnboardingChecklistState::new();

    assert!(state.can_show_hint());
    state.record_hint_shown();
    assert_eq!(state.hints_this_session, 1);

    state.record_hint_shown();
    state.record_hint_shown();
    assert_eq!(state.hints_this_session, 3);
    assert!(!state.can_show_hint());
}

#[test]
fn test_hint_session_reset() {
    let mut state = OnboardingChecklistState::new();

    for _ in 0..MAX_HINTS_PER_SESSION {
        state.record_hint_shown();
    }
    assert!(!state.can_show_hint());

    state.on_session_start();
    assert!(state.can_show_hint());
    assert_eq!(state.hints_this_session, 0);
}

#[test]
fn test_progress_calculation() {
    let mut state = OnboardingChecklistState::new();

    assert_eq!(state.progress(), (0, 5));
    assert_eq!(state.progress_percent(), 0);

    state.mark_milestone(OnboardingMilestone::ProfileReady);
    assert_eq!(state.progress(), (1, 5));
    assert_eq!(state.progress_percent(), 20);

    state.mark_milestone(OnboardingMilestone::ConnectionVerified);
    assert_eq!(state.progress(), (2, 5));
    assert_eq!(state.progress_percent(), 40);
}

#[test]
fn test_reset() {
    let mut state = OnboardingChecklistState::new();
    state.mark_milestone(OnboardingMilestone::ProfileReady);
    state.dismiss_all();
    state.session_count = 5;

    state.reset();

    assert_eq!(state.milestones, OnboardingMilestones::NONE);
    assert!(!state.globally_dismissed);
    assert_eq!(state.session_count, 0);
}

#[test]
fn test_milestone_string_conversion() {
    for milestone in OnboardingMilestone::all() {
        let s = milestone.as_str();
        let restored = OnboardingMilestone::parse(s);
        assert_eq!(Some(milestone), restored);
    }
}

#[test]
fn test_milestone_hint_messages() {
    assert!(!OnboardingMilestone::ProfileReady.hint_message().is_empty());
    assert!(
        !OnboardingMilestone::ConnectionVerified
            .hint_message()
            .is_empty()
    );
    assert!(
        !OnboardingMilestone::FirstSearchRun
            .hint_message()
            .is_empty()
    );
    assert!(
        !OnboardingMilestone::NavigationCycleCompleted
            .hint_message()
            .is_empty()
    );
    assert!(!OnboardingMilestone::HelpOpened.hint_message().is_empty());
}

#[test]
fn test_milestone_titles() {
    assert!(!OnboardingMilestone::ProfileReady.title().is_empty());
    assert!(!OnboardingMilestone::ConnectionVerified.title().is_empty());
    assert!(!OnboardingMilestone::FirstSearchRun.title().is_empty());
    assert!(
        !OnboardingMilestone::NavigationCycleCompleted
            .title()
            .is_empty()
    );
    assert!(!OnboardingMilestone::HelpOpened.title().is_empty());
}

#[test]
fn test_incomplete_milestones_order() {
    let state = OnboardingChecklistState::new();
    let incomplete = state.incomplete_milestones();

    assert_eq!(incomplete.len(), 5);
    assert_eq!(incomplete[0], OnboardingMilestone::ProfileReady);
    assert_eq!(incomplete[1], OnboardingMilestone::ConnectionVerified);
    assert_eq!(incomplete[2], OnboardingMilestone::FirstSearchRun);
    assert_eq!(incomplete[3], OnboardingMilestone::NavigationCycleCompleted);
    assert_eq!(incomplete[4], OnboardingMilestone::HelpOpened);
}

#[test]
fn test_incomplete_milestones_after_completion() {
    let mut state = OnboardingChecklistState::new();
    state.mark_milestone(OnboardingMilestone::ProfileReady);
    state.mark_milestone(OnboardingMilestone::ConnectionVerified);

    let incomplete = state.incomplete_milestones();
    assert_eq!(incomplete.len(), 3);
    assert!(!incomplete.contains(&OnboardingMilestone::ProfileReady));
    assert!(!incomplete.contains(&OnboardingMilestone::ConnectionVerified));
}

#[test]
fn test_session_count_increment() {
    let mut state = OnboardingChecklistState::new();
    assert_eq!(state.session_count, 0);

    state.on_session_start();
    assert_eq!(state.session_count, 1);

    state.on_session_start();
    assert_eq!(state.session_count, 2);
}

#[test]
fn test_sessions_since_completion_increment() {
    let mut state = OnboardingChecklistState::new();

    for m in OnboardingMilestone::all() {
        state.mark_milestone(m);
    }
    assert_eq!(state.sessions_since_completion, 0);

    state.on_session_start();
    assert_eq!(state.sessions_since_completion, 1);
}

#[test]
fn test_should_show_checklist_globally_dismissed() {
    let mut state = OnboardingChecklistState::new();
    state.globally_dismissed = true;

    assert!(!state.should_show_checklist());
}

#[test]
fn test_persistence_roundtrip() {
    let mut state = OnboardingChecklistState::new();
    state.mark_milestone(OnboardingMilestone::ProfileReady);
    state.mark_milestone(OnboardingMilestone::FirstSearchRun);
    state.dismiss_item(&OnboardingMilestone::ConnectionVerified);
    state.session_count = 5;
    state.sessions_since_completion = 2;
    state.globally_dismissed = false;

    let persisted = splunk_config::PersistedOnboardingChecklist {
        milestones: state.milestones.bits(),
        dismissed_items: state.dismissed_items.iter().cloned().collect(),
        session_count: state.session_count,
        sessions_since_completion: state.sessions_since_completion,
        globally_dismissed: state.globally_dismissed,
    };

    let mut restored = OnboardingChecklistState::new();
    restored.milestones = OnboardingMilestones::from_bits_truncate(persisted.milestones);
    restored.dismissed_items = persisted.dismissed_items.into_iter().collect();
    restored.session_count = persisted.session_count;
    restored.sessions_since_completion = persisted.sessions_since_completion;
    restored.globally_dismissed = persisted.globally_dismissed;

    assert_eq!(restored.milestones, state.milestones);
    assert_eq!(restored.session_count, 5);
    assert!(restored.is_item_dismissed(&OnboardingMilestone::ConnectionVerified));
}
