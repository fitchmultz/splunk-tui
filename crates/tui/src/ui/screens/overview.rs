//! Overview screen rendering.
//!
//! Renders a dashboard view of all Splunk resources with counts and status.
//! This provides TUI parity with the CLI's list-all command.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use splunk_config::Theme;

use crate::action::OverviewData;

/// Spinner characters for animated loading indicator.
const SPINNER_CHARS: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

/// Configuration for rendering the overview screen.
pub struct OverviewRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The overview data to display
    pub overview_data: Option<&'a OverviewData>,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the overview screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_overview(f: &mut Frame, area: Rect, config: OverviewRenderConfig) {
    let OverviewRenderConfig {
        loading,
        overview_data,
        theme,
        spinner_frame,
    } = config;

    if loading && overview_data.is_none() {
        let spinner = SPINNER_CHARS[spinner_frame as usize % SPINNER_CHARS.len()];
        let loading_widget = Paragraph::new(format!("{} Loading overview...", spinner))
            .block(Block::default().borders(Borders::ALL).title("Overview"))
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let data = match overview_data {
        Some(d) => d,
        None => {
            let placeholder = Paragraph::new("No overview data loaded. Press 'r' to refresh.")
                .block(Block::default().borders(Borders::ALL).title("Overview"))
                .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    // Create table with resource data
    let header = Row::new(vec!["Resource", "Count", "Status"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .height(1);

    let rows: Vec<Row> = data
        .resources
        .iter()
        .map(|r| {
            let status_color = status_color(&r.status, theme);
            Row::new(vec![
                Cell::from(r.resource_type.clone()),
                Cell::from(r.count.to_string()),
                Cell::from(Span::styled(
                    r.status.clone(),
                    Style::default().fg(status_color),
                )),
            ])
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(40),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Overview - All Resources")
            .border_style(Style::default().fg(theme.border))
            .title_style(Style::default().fg(theme.title)),
    );

    f.render_widget(table, area);
}

fn status_color(status: &str, theme: &Theme) -> ratatui::style::Color {
    match status.to_lowercase().as_str() {
        "ok" | "healthy" | "green" | "active" | "installed" | "available" => theme.success,
        "warning" | "yellow" | "degraded" => theme.warning,
        "error" | "unhealthy" | "red" | "timeout" => theme.error,
        _ => theme.text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{OverviewData, OverviewResource};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_test_overview_data() -> OverviewData {
        OverviewData {
            resources: vec![
                OverviewResource {
                    resource_type: "indexes".to_string(),
                    count: 42,
                    status: "ok".to_string(),
                    error: None,
                },
                OverviewResource {
                    resource_type: "jobs".to_string(),
                    count: 5,
                    status: "active".to_string(),
                    error: None,
                },
                OverviewResource {
                    resource_type: "apps".to_string(),
                    count: 12,
                    status: "installed".to_string(),
                    error: None,
                },
            ],
        }
    }

    #[test]
    fn test_render_overview_with_data() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let data = create_test_overview_data();
        let theme = Theme::default();

        terminal
            .draw(|f| {
                render_overview(
                    f,
                    f.area(),
                    OverviewRenderConfig {
                        loading: false,
                        overview_data: Some(&data),
                        theme: &theme,
                        spinner_frame: 0,
                    },
                );
            })
            .unwrap();

        // Verify the buffer contains expected content
        let buffer = terminal.backend().buffer().clone();
        let content = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("Overview"));
        assert!(content.contains("indexes"));
        assert!(content.contains("42"));
    }

    #[test]
    fn test_render_overview_loading() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|f| {
                render_overview(
                    f,
                    f.area(),
                    OverviewRenderConfig {
                        loading: true,
                        overview_data: None,
                        theme: &theme,
                        spinner_frame: 0,
                    },
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("Loading overview"));
    }

    #[test]
    fn test_render_overview_empty() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|f| {
                render_overview(
                    f,
                    f.area(),
                    OverviewRenderConfig {
                        loading: false,
                        overview_data: None,
                        theme: &theme,
                        spinner_frame: 0,
                    },
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("No overview data loaded"));
    }

    #[test]
    fn test_status_color() {
        let theme = Theme::default();

        // Success statuses
        assert_eq!(status_color("ok", &theme), theme.success);
        assert_eq!(status_color("healthy", &theme), theme.success);
        assert_eq!(status_color("green", &theme), theme.success);
        assert_eq!(status_color("active", &theme), theme.success);
        assert_eq!(status_color("installed", &theme), theme.success);

        // Warning statuses
        assert_eq!(status_color("warning", &theme), theme.warning);
        assert_eq!(status_color("yellow", &theme), theme.warning);

        // Error statuses
        assert_eq!(status_color("error", &theme), theme.error);
        assert_eq!(status_color("unhealthy", &theme), theme.error);
        assert_eq!(status_color("red", &theme), theme.error);
        assert_eq!(status_color("timeout", &theme), theme.error);

        // Unknown status defaults to text color
        assert_eq!(status_color("unknown", &theme), theme.text);
    }
}
