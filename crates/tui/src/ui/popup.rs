//! Modal popup rendering for confirmations and help.

use crate::app::{App, Popup};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Render a modal popup dialog.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `_app` - The application state (unused, reserved for future use)
/// * `popup` - The popup type to render
pub fn render_popup(f: &mut Frame, _app: &App, popup: &Popup) {
    let size = f.area();
    let popup_area = centered_rect(60, 20, size);

    f.render_widget(Clear, popup_area);

    match popup {
        Popup::ConfirmCancel(sid) => {
            let text = format!("Cancel job {sid}? (y/n)");
            let p = Paragraph::new(text.as_str())
                .block(
                    Block::default()
                        .title("Confirm Cancel")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::Red)),
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(p, popup_area);
        }
        Popup::ConfirmDelete(sid) => {
            let text = format!("Delete job {sid}? (y/n)");
            let p = Paragraph::new(text.as_str())
                .block(
                    Block::default()
                        .title("Confirm Delete")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::Red)),
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(p, popup_area);
        }
        Popup::Help => {
            let text = r#"
Global Keys:
  1-4   Navigate screens
  ?     Help
  q     Quit

Search Screen:
  Enter Run search
  j/k   Navigate results

Jobs Screen:
  r     Refresh jobs
  a     Toggle auto-refresh
  c     Cancel job
  d     Delete job
            "#;
            let p = Paragraph::new(text)
                .block(
                    Block::default()
                        .title("Help")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::Cyan)),
                )
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false });
            f.render_widget(p, popup_area);
        }
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
