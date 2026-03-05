//! Keybinding definitions grouped by screen.
//!
//! Responsibilities:
//! - Provide ordered keybinding groups for the keymap resolver.
//!
//! Does NOT handle:
//! - Resolving input events into Actions.
//! - Rendering help or documentation content.
//!
//! Invariants:
//! - Binding order is stable for deterministic help/docs output.

mod global_search;
mod jobs;
mod screens;

use super::Keybinding;

pub(super) fn all() -> Vec<Keybinding> {
    let mut bindings = Vec::new();
    bindings.extend(global_search::bindings());
    bindings.extend(jobs::bindings());
    bindings.extend(screens::bindings());
    bindings
}
