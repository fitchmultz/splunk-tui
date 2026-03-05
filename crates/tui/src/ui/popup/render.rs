//! Popup rendering implementation.
//!
//! Responsibilities:
//! - Render modal popup dialogs with type-specific styling.
//! - Apply popup-type sizing so dense dialogs remain readable.
//!
//! Does NOT handle:
//! - Creating popup content (handled by popup builder).
//! - Handling popup input/actions (handled by app input/actions).
//!
//! Invariants:
//! - Confirmation popups keep default sizing for mouse-hit consistency.
//! - Scroll state for help/tutorial/undo popups is preserved across renders.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use splunk_config::Theme;

use crate::app::App;
use crate::ui::popup::{POPUP_HEIGHT_PERCENT, POPUP_WIDTH_PERCENT, Popup, PopupType};

/// Render a modal popup dialog.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `popup` - The popup to render
/// * `theme` - The color theme to use
/// * `app` - The app state (for accessing scroll offsets)
pub fn render_popup(f: &mut Frame, popup: &Popup, theme: &Theme, app: &App) {
    let size = f.area();
    let (popup_width_pct, popup_height_pct) = popup_size(&popup.kind);
    let popup_area = centered_rect(popup_width_pct, popup_height_pct, size);

    f.render_widget(Clear, popup_area);

    // Determine border color based on popup type
    let border_color = match &popup.kind {
        PopupType::Help
        | PopupType::ExportSearch
        | PopupType::ErrorDetails
        | PopupType::IndexDetails
        | PopupType::ProfileSelector { .. }
        | PopupType::CreateIndex { .. }
        | PopupType::ModifyIndex { .. }
        | PopupType::CreateUser { .. }
        | PopupType::ModifyUser { .. }
        | PopupType::CreateRole { .. }
        | PopupType::ModifyRole { .. }
        | PopupType::InstallAppDialog { .. }
        | PopupType::CreateProfile { .. }
        | PopupType::EditProfile { .. }
        | PopupType::EditSavedSearch { .. }
        | PopupType::CreateSavedSearch { .. }
        | PopupType::CreateMacro { .. }
        | PopupType::EditMacro { .. }
        | PopupType::TutorialWizard { .. }
        | PopupType::CommandPalette { .. }
        | PopupType::UndoHistory { .. }
        | PopupType::ConnectionDiagnostics { .. } => theme.border,
        PopupType::ConfirmCancel(_)
        | PopupType::ConfirmDelete(_)
        | PopupType::ConfirmCancelBatch(_)
        | PopupType::ConfirmDeleteBatch(_)
        | PopupType::ConfirmEnableApp(_)
        | PopupType::ConfirmDisableApp(_)
        | PopupType::DeleteIndexConfirm { .. }
        | PopupType::DeleteUserConfirm { .. }
        | PopupType::DeleteRoleConfirm { .. }
        | PopupType::DeleteSavedSearchConfirm { .. }
        | PopupType::DeleteLookupConfirm { .. }
        | PopupType::ConfirmRemoveApp(_)
        | PopupType::DeleteProfileConfirm { .. }
        | PopupType::AuthRecovery { .. } => theme.error,
    };

    // Determine wrapping behavior based on popup type
    let wrap_mode = match &popup.kind {
        PopupType::Help
        | PopupType::ExportSearch
        | PopupType::ErrorDetails
        | PopupType::IndexDetails
        | PopupType::ProfileSelector { .. }
        | PopupType::CreateIndex { .. }
        | PopupType::ModifyIndex { .. }
        | PopupType::DeleteIndexConfirm { .. }
        | PopupType::CreateUser { .. }
        | PopupType::ModifyUser { .. }
        | PopupType::DeleteUserConfirm { .. }
        | PopupType::CreateRole { .. }
        | PopupType::ModifyRole { .. }
        | PopupType::DeleteRoleConfirm { .. }
        | PopupType::InstallAppDialog { .. }
        | PopupType::CreateProfile { .. }
        | PopupType::EditProfile { .. }
        | PopupType::DeleteProfileConfirm { .. }
        | PopupType::EditSavedSearch { .. }
        | PopupType::CreateSavedSearch { .. }
        | PopupType::CreateMacro { .. }
        | PopupType::EditMacro { .. }
        | PopupType::TutorialWizard { .. }
        | PopupType::CommandPalette { .. }
        | PopupType::UndoHistory { .. }
        | PopupType::AuthRecovery { .. }
        | PopupType::ConnectionDiagnostics { .. } => Wrap { trim: false },
        PopupType::ConfirmCancel(_)
        | PopupType::ConfirmDelete(_)
        | PopupType::ConfirmCancelBatch(_)
        | PopupType::ConfirmDeleteBatch(_)
        | PopupType::ConfirmEnableApp(_)
        | PopupType::ConfirmDisableApp(_)
        | PopupType::ConfirmRemoveApp(_)
        | PopupType::DeleteSavedSearchConfirm { .. }
        | PopupType::DeleteLookupConfirm { .. } => Wrap { trim: true },
    };

    // Determine alignment based on popup type
    // Help popup uses left alignment for better readability of keybindings
    let alignment = match &popup.kind {
        PopupType::Help
        | PopupType::AuthRecovery { .. }
        | PopupType::ConnectionDiagnostics { .. } => Alignment::Left,
        _ => Alignment::Center,
    };

    // For Help popup, apply scroll offset and render scrollbar if needed
    if popup.kind == PopupType::Help {
        let scroll_offset = app.help_scroll_offset;

        let p = Paragraph::new(popup.content.as_str())
            .block(
                Block::default()
                    .title(popup.title.as_str())
                    .borders(Borders::ALL)
                    .style(Style::default().fg(border_color)),
            )
            .alignment(alignment)
            .wrap(wrap_mode)
            .scroll((scroll_offset as u16, 0));
        f.render_widget(p, popup_area);

        // Calculate content height and render scrollbar if needed
        // Content height is the number of lines in the content
        let content_height = popup.content.lines().count();
        let visible_lines = popup_area.height.saturating_sub(2) as usize; // Account for borders

        if content_height > visible_lines {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state =
                ScrollbarState::new(content_height.saturating_sub(1)).position(scroll_offset);
            f.render_stateful_widget(
                scrollbar,
                popup_area.inner(Margin::new(0, 1)),
                &mut scrollbar_state,
            );
        }
    } else if let PopupType::TutorialWizard { state } = &popup.kind {
        // Tutorial wizard popup with scroll support for keybinding step
        let scroll_offset = state.keybinding_scroll_offset;

        let p = Paragraph::new(popup.content.as_str())
            .block(
                Block::default()
                    .title(popup.title.as_str())
                    .borders(Borders::ALL)
                    .style(Style::default().fg(border_color)),
            )
            .alignment(alignment)
            .wrap(wrap_mode)
            .scroll((scroll_offset as u16, 0));
        f.render_widget(p, popup_area);

        // Calculate content height and render scrollbar if needed
        let content_height = popup.content.lines().count();
        let visible_lines = popup_area.height.saturating_sub(2) as usize; // Account for borders

        if content_height > visible_lines {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state =
                ScrollbarState::new(content_height.saturating_sub(1)).position(scroll_offset);
            f.render_stateful_widget(
                scrollbar,
                popup_area.inner(Margin::new(0, 1)),
                &mut scrollbar_state,
            );
        }
    } else if let PopupType::UndoHistory { scroll_offset } = &popup.kind {
        // Undo history popup with scroll support
        let scroll_offset = *scroll_offset;

        // Build the content from undo buffer history
        let mut content =
            String::from("Recent operations (Ctrl+Z to undo, Ctrl+Shift+Z to redo):\n\n");

        if app.undo_buffer.history().is_empty() {
            content.push_str("No operations in history.");
        } else {
            for (i, entry) in app.undo_buffer.history().iter().enumerate() {
                let status_icon = match entry.status() {
                    crate::undo::UndoEntryStatus::Pending => "⏳",
                    crate::undo::UndoEntryStatus::Executed => "✓",
                    crate::undo::UndoEntryStatus::Undone => "↩",
                    crate::undo::UndoEntryStatus::Expired => "✗",
                };
                let remaining = if entry.status() == crate::undo::UndoEntryStatus::Pending {
                    format!(" ({}s)", entry.remaining_secs())
                } else {
                    String::new()
                };
                content.push_str(&format!(
                    "{} {}: {}{}\n",
                    status_icon,
                    i + 1,
                    entry.description,
                    remaining
                ));
            }
        }

        // Add redo stack if not empty
        if !app.undo_buffer.redo_stack().is_empty() {
            content.push_str("\nRedo stack:\n");
            for (i, entry) in app.undo_buffer.redo_stack().iter().enumerate() {
                content.push_str(&format!("  {}: {}\n", i + 1, entry.description));
            }
        }

        content.push_str("\nPress Esc to close, j/k to scroll");

        let p = Paragraph::new(content.as_str())
            .block(
                Block::default()
                    .title(popup.title.as_str())
                    .borders(Borders::ALL)
                    .style(Style::default().fg(border_color)),
            )
            .alignment(Alignment::Left)
            .wrap(wrap_mode)
            .scroll((scroll_offset as u16, 0));
        f.render_widget(p, popup_area);

        // Calculate content height and render scrollbar if needed
        let content_height = content.lines().count();
        let visible_lines = popup_area.height.saturating_sub(2) as usize; // Account for borders

        if content_height > visible_lines {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state =
                ScrollbarState::new(content_height.saturating_sub(1)).position(scroll_offset);
            f.render_stateful_widget(
                scrollbar,
                popup_area.inner(Margin::new(0, 1)),
                &mut scrollbar_state,
            );
        }
    } else {
        let p = Paragraph::new(popup.content.as_str())
            .block(
                Block::default()
                    .title(popup.title.as_str())
                    .borders(Borders::ALL)
                    .style(Style::default().fg(border_color)),
            )
            .alignment(alignment)
            .wrap(wrap_mode);
        f.render_widget(p, popup_area);
    }
}

fn popup_size(kind: &PopupType) -> (u16, u16) {
    match kind {
        // Dense, scroll-heavy popups need more room for readability.
        PopupType::Help => (78, 76),
        PopupType::TutorialWizard { .. } => (72, 72),
        PopupType::CreateProfile { .. }
        | PopupType::EditProfile { .. }
        | PopupType::ProfileSelector { .. } => (72, 62),
        PopupType::ConnectionDiagnostics { .. } => (70, 58),
        _ => (POPUP_WIDTH_PERCENT, POPUP_HEIGHT_PERCENT),
    }
}

/// Create a centered rectangle with the given percentage of the screen size.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
