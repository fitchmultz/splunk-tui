//! Shared screen-state rendering helpers for data-driven screens.
//!
//! Responsibilities:
//! - Render standard loading placeholders for screens awaiting data.
//! - Render standard empty placeholders when data is absent.
//! - Return loaded data so screens can focus on ready-state rendering only.
//!
//! Does NOT handle:
//! - Rendering the ready state.
//! - Special empty-state messaging beyond the shared default.
//! - Selection or layout logic.
//!
//! Invariants:
//! - Loading placeholders render only when the screen is still waiting on data.
//! - Empty placeholders render only when no data exists.

use ratatui::{Frame, layout::Rect};
use splunk_config::Theme;

use super::{render_empty_state, render_empty_state_custom, render_loading_state};

#[allow(clippy::too_many_arguments)]
pub fn render_screen_state<'a, T: ?Sized>(
    f: &mut Frame,
    area: Rect,
    loading: bool,
    data: Option<&'a T>,
    title: &str,
    loading_message: &str,
    empty_label: &str,
    spinner_frame: u8,
    theme: &Theme,
) -> Option<&'a T> {
    if loading && data.is_none() {
        render_loading_state(f, area, title, loading_message, spinner_frame, theme);
        return None;
    }

    match data {
        Some(data) => Some(data),
        None => {
            render_empty_state(f, area, title, empty_label);
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_screen_state_custom<'a, T: ?Sized>(
    f: &mut Frame,
    area: Rect,
    loading: bool,
    data: Option<&'a T>,
    title: &str,
    loading_message: &str,
    empty_message: &str,
    spinner_frame: u8,
    theme: &Theme,
) -> Option<&'a T> {
    if loading && data.is_none() {
        render_loading_state(f, area, title, loading_message, spinner_frame, theme);
        return None;
    }

    match data {
        Some(data) => Some(data),
        None => {
            render_empty_state_custom(f, area, title, empty_message);
            None
        }
    }
}
