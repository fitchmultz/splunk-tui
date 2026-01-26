//! Error details popup rendering with structured display.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarState, Wrap},
};

use crate::app::App;
use crate::error_details::ErrorDetails;
use splunk_config::Theme;

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

pub fn render_error_details(f: &mut Frame, error: &ErrorDetails, app: &App, theme: &Theme) {
    let area = f.area();

    let popup_width = 80.min(area.width.saturating_sub(4));
    let popup_height = 25.min(area.height.saturating_sub(4));
    let mut popup_area = centered_rect(80, 25, area);
    popup_area.width = popup_width;
    popup_area.height = popup_height;

    f.render_widget(Clear, popup_area);

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Summary: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&error.summary),
        ]),
        Line::default(),
    ];

    if let Some(status) = error.status_code {
        lines.push(Line::from(vec![
            Span::styled(
                "Status Code: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}", status),
                Style::default().fg(if status >= 500 {
                    theme.error
                } else {
                    theme.warning
                }),
            ),
        ]));
        lines.push(Line::default());
    }

    if let Some(url) = &error.url {
        lines.push(Line::from(vec![
            Span::styled("URL: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(url),
        ]));
        lines.push(Line::default());
    }

    if let Some(rid) = &error.request_id {
        lines.push(Line::from(vec![
            Span::styled(
                "Request ID: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(rid, Style::default().fg(theme.accent)),
        ]));
        lines.push(Line::default());
    }

    if !error.context.is_empty() {
        lines.push(Line::from(Span::styled(
            "Context:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme.title),
        )));

        for (key, value) in &error.context {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{}: ", key),
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(theme.accent),
                ),
                Span::raw(value),
            ]));
        }
        lines.push(Line::default());
    }

    if !error.messages.is_empty() {
        lines.push(Line::from(Span::styled(
            "Splunk Messages:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme.title),
        )));

        for msg in &error.messages {
            let color = match msg.message_type.as_str() {
                "ERROR" => theme.error,
                "WARN" => theme.warning,
                "INFO" => theme.info,
                _ => theme.text_dim,
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  [{}] ", msg.message_type),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(&msg.text),
            ]));
        }
        lines.push(Line::default());
    }

    if let Some(body) = &error.raw_body {
        lines.push(Line::from(Span::styled(
            "Raw Response:",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) {
            let pretty = serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| body.to_string());
            let pretty_lines: Vec<Line> = pretty
                .lines()
                .map(|l| Line::from(Span::raw(l.to_string())))
                .collect();
            lines.extend(pretty_lines);
        } else {
            let body_lines: Vec<Line> = body
                .lines()
                .map(|l| Line::from(Span::raw(l.to_string())))
                .collect();
            lines.extend(body_lines);
        }
    }

    let paragraph = Paragraph::new(lines.clone())
        .block(
            Block::default()
                .title("Error Details")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.error)),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.error_scroll_offset as u16, 0));

    f.render_widget(paragraph, popup_area);

    let content_height = lines.len();
    let visible_lines = popup_height.saturating_sub(2) as usize;

    if content_height > visible_lines {
        let scrollbar = Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state =
            ScrollbarState::new(content_height.saturating_sub(1)).position(app.error_scroll_offset);
        f.render_stateful_widget(
            scrollbar,
            popup_area.inner(Margin::new(0, 1)),
            &mut scrollbar_state,
        );
    }
}
