//! Settings screen rendering module.
//!
//! Responsibilities:
//! - Render runtime settings values and profile context.
//! - Render concise, accurate shortcut guidance for settings actions.
//!
//! Does NOT handle:
//! - Processing key input (handled in `app/input/settings.rs`).
//! - Persisting settings state (handled by actions/data-loading modules).
//!
//! Invariants:
//! - Displayed key hints must match actual settings keybindings.
//! - Styling flows through the shared `ThemeExt` theme helpers.

use crate::ui::theme::ThemeExt;
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
    pub max_results: usize,
    /// Default count for internal logs queries.
    pub internal_logs_count: usize,
    /// Default earliest time for internal logs queries.
    pub internal_logs_earliest: &'a str,
}

fn settings_shortcut_rows() -> [&'static str; 4] {
    [
        "t:Diagnostics  T:Theme  a:Auto-refresh",
        "s:Sort column  d:Direction  c:Clear history",
        "p:Switch profile  n:Create  e:Edit  x:Delete",
        "u:Undo history  ?:Replay tutorial",
    ]
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

    let shortcut_rows = settings_shortcut_rows();
    let compact_layout = chunks[1].height <= 20;

    // Content lines - using ThemeExt for consistent styling.
    let mut content = vec![Line::from(vec![
        Span::styled("Theme:          ", theme.title()),
        Span::styled(config.selected_theme.to_string(), theme.text()),
    ])];

    if compact_layout {
        content.push(Line::from(vec![
            Span::styled("Auto-refresh:   ", theme.title()),
            Span::styled(&auto_refresh_text, auto_refresh_style),
            Span::styled("  Sort: ", theme.title()),
            Span::styled(config.sort_column, theme.text()),
            Span::styled("/", theme.text_dim()),
            Span::styled(config.sort_direction, theme.text()),
        ]));
        content.push(Line::from(vec![
            Span::styled("Profile:        ", theme.title()),
            Span::styled(profile_display, theme.text()),
            Span::styled("  History: ", theme.title()),
            Span::styled(format!("{}", config.search_history_count), theme.text()),
        ]));
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("Search defaults: ", theme.title()),
            Span::styled(
                format!(
                    "{} → {} (max {})",
                    config.earliest_time, config.latest_time, config.max_results
                ),
                theme.text(),
            ),
        ]));
        content.push(Line::from(vec![
            Span::styled("Internal logs:   ", theme.title()),
            Span::styled(
                format!(
                    "count {} / earliest {}",
                    config.internal_logs_count, config.internal_logs_earliest
                ),
                theme.text(),
            ),
        ]));
    } else {
        // Theme preview showing key semantic colors
        content.push(Line::from(vec![
            Span::styled("                ", theme.text()), // Indent to align with theme value
            Span::styled("█", Style::default().fg(theme.success)),
            Span::styled(" success  ", theme.text_dim()),
            Span::styled("█", Style::default().fg(theme.warning)),
            Span::styled(" warning  ", theme.text_dim()),
            Span::styled("█", Style::default().fg(theme.error)),
            Span::styled(" error  ", theme.text_dim()),
            Span::styled("█", Style::default().fg(theme.info)),
            Span::styled(" info", theme.text_dim()),
        ]));
        content.push(Line::from(vec![
            Span::styled("Auto-refresh:   ", theme.title()),
            Span::styled(&auto_refresh_text, auto_refresh_style),
        ]));
        content.push(Line::from(vec![
            Span::styled("Sort column:    ", theme.title()),
            Span::styled(config.sort_column, theme.text()),
        ]));
        content.push(Line::from(vec![
            Span::styled("Sort direction: ", theme.title()),
            Span::styled(config.sort_direction, theme.text()),
        ]));
        content.push(Line::from(vec![
            Span::styled("Search history: ", theme.title()),
            Span::styled(
                format!("{} items", config.search_history_count),
                theme.text(),
            ),
        ]));
        content.push(Line::from(vec![
            Span::styled("Profile:        ", theme.title()),
            Span::styled(profile_display, theme.text()),
        ]));
        content.push(Line::from(""));
        content.push(Line::from(vec![Span::styled(
            "Search Defaults",
            theme.title(),
        )]));
        content.push(Line::from(vec![
            Span::styled("  Earliest time: ", theme.title()),
            Span::styled(config.earliest_time, theme.text()),
        ]));
        content.push(Line::from(vec![
            Span::styled("  Latest time:   ", theme.title()),
            Span::styled(config.latest_time, theme.text()),
        ]));
        content.push(Line::from(vec![
            Span::styled("  Max results:   ", theme.title()),
            Span::styled(format!("{}", config.max_results), theme.text()),
        ]));
        content.push(Line::from(""));
        content.push(Line::from(vec![Span::styled(
            "Internal Logs Defaults",
            theme.title(),
        )]));
        content.push(Line::from(vec![
            Span::styled("  Count:         ", theme.title()),
            Span::styled(format!("{}", config.internal_logs_count), theme.text()),
        ]));
        content.push(Line::from(vec![
            Span::styled("  Earliest time: ", theme.title()),
            Span::styled(config.internal_logs_earliest, theme.text()),
        ]));
    }

    content.push(Line::from(""));
    content.push(Line::from(vec![Span::styled("Shortcuts", theme.title())]));
    content.push(Line::from(vec![Span::styled(
        shortcut_rows[0],
        theme.text_dim(),
    )]));
    content.push(Line::from(vec![Span::styled(
        shortcut_rows[1],
        theme.text_dim(),
    )]));
    content.push(Line::from(vec![Span::styled(
        shortcut_rows[2],
        theme.text_dim(),
    )]));
    content.push(Line::from(vec![Span::styled(
        shortcut_rows[3],
        theme.text_dim(),
    )]));

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

#[cfg(test)]
mod tests {
    use super::settings_shortcut_rows;

    #[test]
    fn test_shortcut_rows_include_current_settings_keys() {
        let rows = settings_shortcut_rows();
        let joined = rows.join(" | ");

        assert!(joined.contains("t:Diagnostics"));
        assert!(joined.contains("T:Theme"));
        assert!(joined.contains("u:Undo history"));
        assert!(joined.contains("?:Replay tutorial"));
    }
}
