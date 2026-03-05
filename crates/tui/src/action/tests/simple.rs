//! Tests for simple action redaction.

use crate::action::tests::redacted_debug;
use crate::action::variants::Action;

#[test]
fn test_show_search_input() {
    let action = Action::SearchInput('s');
    let output = redacted_debug(&action);

    assert!(output.contains("SearchInput"), "Should contain action name");
    assert!(
        output.contains("'s'"),
        "Should show character for input debugging"
    );
}

#[test]
fn test_non_sensitive_action_shown_fully() {
    let action = Action::Quit;
    let output = redacted_debug(&action);

    assert!(output.contains("Quit"), "Should show simple action fully");
}
