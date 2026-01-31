//! Settings screen rendering module.
//!
//! This module provides the UI for viewing and modifying runtime configuration.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
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
    /// Selected persisted theme (for display).
    pub selected_theme: splunk_config::ColorTheme,
    /// Runtime expanded theme (for colors).
    pub theme: &'a splunk_config::Theme,
    /// Default earliest time for searches (e.g., "-24h").
    pub earliest_time: &'a str,
    /// Default latest time for searches (e.g., "now").
    pub latest_time: &'a str,
    /// Default maximum number of results per search.
    pub max_results: u64,
    /// Default count for internal logs queries.
    pub internal_logs_count: u64,
    /// Default earliest time for internal logs queries.
    pub internal_logs_earliest: &'a str,
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

    let theme = config.theme;

    // Header
    let header = Line::from(vec![Span::styled(
        "Settings",
        Style::default().fg(theme.accent),
    )]);

    f.render_widget(
        Paragraph::new(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(theme.border)),
            )
            .alignment(Alignment::Center),
        chunks[0],
    );

    // Content
    let auto_refresh_color = if config.auto_refresh {
        theme.success
    } else {
        theme.warning
    };

    let auto_refresh_text = format!("[{}]", if config.auto_refresh { "On" } else { "Off" });

    let profile_display = config.profile_info.unwrap_or("N/A");

    let content = vec![
        Line::from(vec![
            Span::styled("Theme:          ", Style::default().fg(theme.title)),
            Span::styled(
                config.selected_theme.to_string(),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("Auto-refresh:   ", Style::default().fg(theme.title)),
            Span::styled(&auto_refresh_text, Style::default().fg(auto_refresh_color)),
        ]),
        Line::from(vec![
            Span::styled("Sort column:    ", Style::default().fg(theme.title)),
            Span::styled(config.sort_column, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("Sort direction: ", Style::default().fg(theme.title)),
            Span::styled(config.sort_direction, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("Search history: ", Style::default().fg(theme.title)),
            Span::styled(
                format!("{} items", config.search_history_count),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("Profile:        ", Style::default().fg(theme.title)),
            Span::styled(profile_display, Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Search Defaults",
            Style::default().fg(theme.title),
        )]),
        Line::from(vec![
            Span::styled("  Earliest time: ", Style::default().fg(theme.title)),
            Span::styled(config.earliest_time, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Latest time:   ", Style::default().fg(theme.title)),
            Span::styled(config.latest_time, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Max results:   ", Style::default().fg(theme.title)),
            Span::styled(
                format!("{}", config.max_results),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Internal Logs Defaults",
            Style::default().fg(theme.title),
        )]),
        Line::from(vec![
            Span::styled("  Count:         ", Style::default().fg(theme.title)),
            Span::styled(
                format!("{}", config.internal_logs_count),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Earliest time: ", Style::default().fg(theme.title)),
            Span::styled(
                config.internal_logs_earliest,
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("t", Style::default().fg(theme.accent)),
            Span::styled("' to cycle theme", Style::default().fg(theme.text_dim)),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("a", Style::default().fg(theme.accent)),
            Span::styled(
                "' to toggle auto-refresh",
                Style::default().fg(theme.text_dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("s", Style::default().fg(theme.accent)),
            Span::styled(
                "' to cycle sort column",
                Style::default().fg(theme.text_dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("d", Style::default().fg(theme.accent)),
            Span::styled(
                "' to toggle sort direction",
                Style::default().fg(theme.text_dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("c", Style::default().fg(theme.accent)),
            Span::styled(
                "' to clear search history",
                Style::default().fg(theme.text_dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("r", Style::default().fg(theme.accent)),
            Span::styled(
                "' to reload settings file",
                Style::default().fg(theme.text_dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("p", Style::default().fg(theme.accent)),
            Span::styled("' to switch profile", Style::default().fg(theme.text_dim)),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("n", Style::default().fg(theme.accent)),
            Span::styled(
                "' to create new profile",
                Style::default().fg(theme.text_dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("e", Style::default().fg(theme.accent)),
            Span::styled(
                "' to edit selected profile",
                Style::default().fg(theme.text_dim),
            ),
        ]),
        Line::from(vec![
            Span::styled("Press '", Style::default().fg(theme.text_dim)),
            Span::styled("x", Style::default().fg(theme.accent)),
            Span::styled(
                "' to delete selected profile",
                Style::default().fg(theme.text_dim),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(theme.border)),
            )
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left),
        chunks[1],
    );
}
