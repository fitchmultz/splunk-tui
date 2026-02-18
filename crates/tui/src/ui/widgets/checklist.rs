//! Onboarding checklist widget for progressive user guidance.
//!
//! Renders a compact widget showing onboarding progress with dismissible
//! individual items and global dismiss option.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use splunk_config::Theme;

use crate::onboarding::checklist::OnboardingChecklistState;

pub fn render_onboarding_checklist(
    f: &mut Frame,
    state: &OnboardingChecklistState,
    area: Rect,
    theme: &Theme,
) {
    if !state.should_show_checklist() {
        return;
    }

    if area.width == 0 || area.height == 0 {
        return;
    }

    let (completed, total) = state.progress();
    let percent = state.progress_percent();

    let title_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);

    let mut lines = vec![Line::from(vec![
        Span::styled(" Onboarding ", title_style),
        Span::styled(
            format!("[{}/{}]", completed, total),
            Style::default().fg(theme.text),
        ),
        Span::styled(
            format!(" {}%", percent),
            Style::default().fg(if percent == 100 {
                theme.success
            } else {
                theme.text_dim
            }),
        ),
    ])];

    let incomplete: Vec<_> = state.incomplete_milestones();
    for milestone in incomplete.iter().take(3) {
        let check = if state.milestones.contains(milestone.to_flag()) {
            "✓"
        } else {
            "○"
        };
        let style = if state.milestones.contains(milestone.to_flag()) {
            Style::default().fg(theme.success)
        } else {
            Style::default().fg(theme.text_dim)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", check), style),
            Span::styled(milestone.title(), Style::default().fg(theme.text)),
        ]));
    }

    lines.push(Line::from(vec![Span::styled(
        " Press 'd' to dismiss ",
        Style::default().fg(theme.text_dim),
    )]));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
    );

    f.render_widget(paragraph, area);
}

pub fn checklist_area(frame_area: Rect) -> Rect {
    const CHECKLIST_WIDTH: u16 = 28;
    const CHECKLIST_HEIGHT: u16 = 6;
    const FOOTER_HEIGHT: u16 = 3;
    const MIN_WIDTH: u16 = CHECKLIST_WIDTH + 2;
    const MIN_HEIGHT: u16 = FOOTER_HEIGHT + CHECKLIST_HEIGHT + 1;

    if frame_area.width < MIN_WIDTH || frame_area.height < MIN_HEIGHT {
        return Rect::default();
    }

    Rect {
        x: frame_area.width.saturating_sub(CHECKLIST_WIDTH + 2),
        y: frame_area
            .height
            .saturating_sub(FOOTER_HEIGHT + CHECKLIST_HEIGHT + 1),
        width: CHECKLIST_WIDTH,
        height: CHECKLIST_HEIGHT,
    }
}
