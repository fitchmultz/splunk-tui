//! Overview screen rendering.
//!
//! Renders a dashboard view of all Splunk resources with counts and status.
//! This provides TUI parity with the CLI's list-all command.

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::Style,
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table},
};
use splunk_config::Theme;

use crate::action::OverviewData;
use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};

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
        render_loading_state(
            f,
            area,
            "Overview",
            "Loading overview...",
            spinner_frame,
            theme,
        );
        return;
    }

    let data = match overview_data {
        Some(d) => d,
        None => {
            render_empty_state(f, area, "Overview", "overview data");
            return;
        }
    };

    // Create table with resource data
    let header = Row::new(vec![
        Cell::from("Resource").style(theme.table_header()),
        Cell::from("Count").style(theme.table_header()),
        Cell::from("Status").style(theme.table_header()),
    ])
    .height(1);

    let rows: Vec<Row> = data
        .resources
        .iter()
        .map(|r| {
            let status_color = theme.status_color(&r.status);
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
            .border_style(theme.border())
            .title_style(theme.title()),
    );

    f.render_widget(table, area);
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
        assert_eq!(theme.status_color("ok"), theme.success);
        assert_eq!(theme.status_color("healthy"), theme.success);
        assert_eq!(theme.status_color("green"), theme.success);
        assert_eq!(theme.status_color("active"), theme.success);
        assert_eq!(theme.status_color("installed"), theme.success);

        // Warning statuses
        assert_eq!(theme.status_color("warning"), theme.warning);
        assert_eq!(theme.status_color("yellow"), theme.warning);

        // Error statuses
        assert_eq!(theme.status_color("error"), theme.error);
        assert_eq!(theme.status_color("unhealthy"), theme.error);
        assert_eq!(theme.status_color("red"), theme.error);
        assert_eq!(theme.status_color("timeout"), theme.error);

        // Unknown status defaults to text color
        assert_eq!(theme.status_color("unknown"), theme.text);
    }
}
