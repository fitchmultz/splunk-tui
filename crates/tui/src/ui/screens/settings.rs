//! Settings screen rendering module.
//!
//! This module provides the UI for viewing and modifying runtime configuration.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render configuration for the settings screen.
pub struct SettingsRenderConfig<'a> {
    /// Current auto-refresh state
    pub auto_refresh: bool,
    /// Current sort column
    pub sort_column: &'a str,
    /// Current sort direction
    pub sort_direction: &'a str,
    /// Number of items in search history
    pub search_history_count: usize,
    /// Current profile name (if SPLUNK_PROFILE is set)
    pub profile_info: Option<&'a str>,
}

/// Render the settings screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration data for the settings screen
pub fn render_settings(f: &mut Frame, area: Rect, config: SettingsRenderConfig) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // Header
    let header = Line::from(vec![Span::styled(
        "Settings",
        Style::default().fg(Color::Yellow),
    )]);

    f.render_widget(
        Paragraph::new(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Center),
        chunks[0],
    );

    // Content
    let auto_refresh_color = if config.auto_refresh {
        Color::Green
    } else {
        Color::Yellow
    };

    let auto_refresh_text = format!("[{}]", if config.auto_refresh { "On" } else { "Off" });

    let profile_display = config.profile_info.unwrap_or("N/A");

    let content = vec![
        Line::from(vec![
            Span::styled("Auto-refresh:  ", Style::default().fg(Color::Cyan)),
            Span::styled(&auto_refresh_text, Style::default().fg(auto_refresh_color)),
        ]),
        Line::from(vec![
            Span::styled("Sort column:    ", Style::default().fg(Color::Cyan)),
            Span::styled(config.sort_column, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Sort direction: ", Style::default().fg(Color::Cyan)),
            Span::styled(config.sort_direction, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Search history: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{} items", config.search_history_count),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Profile:        ", Style::default().fg(Color::Cyan)),
            Span::styled(profile_display, Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(Color::Yellow)),
            Span::styled("a", Style::default().fg(Color::Green)),
            Span::styled("' to toggle auto-refresh", Style::default()),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(Color::Yellow)),
            Span::styled("s", Style::default().fg(Color::Green)),
            Span::styled("' to cycle sort column", Style::default()),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(Color::Yellow)),
            Span::styled("d", Style::default().fg(Color::Green)),
            Span::styled("' to toggle sort direction", Style::default()),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(Color::Yellow)),
            Span::styled("c", Style::default().fg(Color::Green)),
            Span::styled("' to clear search history", Style::default()),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(Color::Yellow)),
            Span::styled("r", Style::default().fg(Color::Green)),
            Span::styled("' to reload settings file", Style::default()),
        ]),
    ];

    f.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Cyan)),
            )
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left),
        chunks[1],
    );
}
