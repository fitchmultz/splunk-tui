//! Settings screen rendering module.
//!
//! This module provides the UI for viewing and modifying runtime configuration.
//!
//! Uses the centralized theme system via [`ThemeExt`] for consistent styling.

use crate::ui::theme::ThemeExt;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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
    pub max_results: usize,
    /// Default count for internal logs queries.
    pub internal_logs_count: usize,
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

    // Header - using ThemeExt for consistent title styling
    let header = Line::from(vec![Span::styled("Settings", theme.title())]);

    f.render_widget(
        Paragraph::new(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border()),
            )
            .alignment(Alignment::Center),
        chunks[0],
    );

    // Content - using ThemeExt for semantic color styling
    let auto_refresh_style = if config.auto_refresh {
        theme.success()
    } else {
        theme.warning()
    };

    let auto_refresh_text = format!("[{}]", if config.auto_refresh { "On" } else { "Off" });

    let profile_display = config.profile_info.unwrap_or("N/A");

    // Content lines - using ThemeExt for consistent styling
    let content = vec![
        Line::from(vec![
            Span::styled("Theme:          ", theme.title()),
            Span::styled(config.selected_theme.to_string(), theme.text()),
        ]),
        Line::from(vec![
            Span::styled("Auto-refresh:   ", theme.title()),
            Span::styled(&auto_refresh_text, auto_refresh_style),
        ]),
        Line::from(vec![
            Span::styled("Sort column:    ", theme.title()),
            Span::styled(config.sort_column, theme.text()),
        ]),
        Line::from(vec![
            Span::styled("Sort direction: ", theme.title()),
            Span::styled(config.sort_direction, theme.text()),
        ]),
        Line::from(vec![
            Span::styled("Search history: ", theme.title()),
            Span::styled(
                format!("{} items", config.search_history_count),
                theme.text(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Profile:        ", theme.title()),
            Span::styled(profile_display, theme.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Search Defaults", theme.title())]),
        Line::from(vec![
            Span::styled("  Earliest time: ", theme.title()),
            Span::styled(config.earliest_time, theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  Latest time:   ", theme.title()),
            Span::styled(config.latest_time, theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  Max results:   ", theme.title()),
            Span::styled(format!("{}", config.max_results), theme.text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Internal Logs Defaults", theme.title())]),
        Line::from(vec![
            Span::styled("  Count:         ", theme.title()),
            Span::styled(format!("{}", config.internal_logs_count), theme.text()),
        ]),
        Line::from(vec![
            Span::styled("  Earliest time: ", theme.title()),
            Span::styled(config.internal_logs_earliest, theme.text()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("t", theme.title()),
            Span::styled("' to cycle theme", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("a", theme.title()),
            Span::styled("' to toggle auto-refresh", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("s", theme.title()),
            Span::styled("' to cycle sort column", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("d", theme.title()),
            Span::styled("' to toggle sort direction", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("c", theme.title()),
            Span::styled("' to clear search history", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("r", theme.title()),
            Span::styled("' to reload settings file", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("p", theme.title()),
            Span::styled("' to switch profile", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("n", theme.title()),
            Span::styled("' to create new profile", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("e", theme.title()),
            Span::styled("' to edit selected profile", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("x", theme.title()),
            Span::styled("' to delete selected profile", theme.text_dim()),
        ]),
        Line::from(vec![
            Span::styled("Press '", theme.text_dim()),
            Span::styled("?", theme.title()),
            Span::styled("' to replay the tutorial", theme.text_dim()),
        ]),
    ];

    f.render_widget(
        Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.border()),
            )
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left),
        chunks[1],
    );
}
