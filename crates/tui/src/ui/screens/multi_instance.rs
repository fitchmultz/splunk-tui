//! Multi-instance dashboard screen rendering.
//!
//! Renders a two-panel layout showing aggregated health/status from multiple
//! Splunk profiles. The left panel shows the instance list with health indicators,
//! and the right panel shows detailed resource information for the selected instance.
//!
//! This provides TUI parity with the CLI's list-all --all-profiles command.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use splunk_config::Theme;

use crate::action::MultiInstanceOverviewData;

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};

/// Configuration for rendering the multi-instance dashboard.
pub struct MultiInstanceRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The multi-instance data to display
    pub data: Option<&'a MultiInstanceOverviewData>,
    /// Currently selected instance index
    pub selected_index: usize,
    /// Theme for consistent styling
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the multi-instance dashboard screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_multi_instance(f: &mut Frame, area: Rect, config: MultiInstanceRenderConfig) {
    let MultiInstanceRenderConfig {
        loading,
        data,
        selected_index,
        theme,
        spinner_frame,
    } = config;

    if loading && data.is_none() {
        render_loading_state(
            f,
            area,
            "Multi-Instance",
            "Loading multi-instance dashboard...",
            spinner_frame,
            theme,
        );
        return;
    }

    let overview_data = match data {
        Some(d) => d,
        None => {
            render_empty_state(f, area, "Multi-Instance", "multi-instance data");
            return;
        }
    };

    // Split into two panels: left (instance list) and right (details)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let left_panel = chunks[0];
    let right_panel = chunks[1];

    // Render left panel: instance list
    render_instance_list(f, left_panel, overview_data, selected_index, theme);

    // Render right panel: instance details
    render_instance_details(f, right_panel, overview_data, selected_index, theme);
}

/// Render the left panel with the list of instances.
fn render_instance_list(
    f: &mut Frame,
    area: Rect,
    data: &MultiInstanceOverviewData,
    selected_index: usize,
    theme: &Theme,
) {
    use crate::action::InstanceStatus;

    let header = Row::new(vec![
        Cell::from("Instance").style(theme.table_header()),
        Cell::from("Status").style(theme.table_header()),
        Cell::from("Jobs").style(theme.table_header()),
    ])
    .height(1);

    let rows: Vec<Row> = data
        .instances
        .iter()
        .enumerate()
        .map(|(idx, instance)| {
            let is_selected = idx == selected_index;

            let (status_text, status_color) = match instance.status {
                InstanceStatus::Healthy => (
                    instance.health_status.clone(),
                    theme.status_color(&instance.health_status),
                ),
                InstanceStatus::Cached => ("Cached".to_string(), theme.warning),
                InstanceStatus::Failed => ("Failed".to_string(), theme.error),
                InstanceStatus::Loading => ("Loading...".to_string(), theme.info),
            };

            let row_style = if is_selected {
                theme.highlight()
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(instance.profile_name.clone()),
                Cell::from(Span::styled(status_text, Style::default().fg(status_color))),
                Cell::from(instance.job_count.to_string()),
            ])
            .height(1)
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Instances ({})", data.instances.len()))
            .border_style(theme.border())
            .title_style(theme.title()),
    );

    f.render_widget(table, area);
}

/// Render the right panel with details for the selected instance.
fn render_instance_details(
    f: &mut Frame,
    area: Rect,
    data: &MultiInstanceOverviewData,
    selected_index: usize,
    theme: &Theme,
) {
    use crate::action::InstanceStatus;

    let instance = match data.instances.get(selected_index) {
        Some(i) => i,
        None => {
            let placeholder = Paragraph::new("No instance selected")
                .block(Block::default().borders(Borders::ALL).title("Details"))
                .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    let status_str = match instance.status {
        InstanceStatus::Healthy => "Healthy",
        InstanceStatus::Cached => "Cached (Instance Unreachable)",
        InstanceStatus::Failed => "Failed",
        InstanceStatus::Loading => "Loading...",
    };

    // Build the content
    let mut text_lines: Vec<Line> = vec![
        // Instance header
        Line::from(vec![
            Span::styled("Instance: ", theme.title()),
            Span::raw(&instance.profile_name),
        ]),
        Line::from(vec![
            Span::styled("URL:      ", theme.title()),
            Span::raw(&instance.base_url),
        ]),
        Line::from(vec![
            Span::styled("Status:   ", theme.title()),
            Span::styled(
                status_str,
                Style::default().fg(match instance.status {
                    InstanceStatus::Healthy => theme.success,
                    InstanceStatus::Cached => theme.warning,
                    InstanceStatus::Failed => theme.error,
                    InstanceStatus::Loading => theme.info,
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Health:   ", theme.title()),
            Span::styled(
                &instance.health_status,
                Style::default().fg(theme.status_color(&instance.health_status)),
            ),
        ]),
    ];

    if let Some(ref last_success) = instance.last_success_at {
        text_lines.push(Line::from(vec![
            Span::styled("Last Success: ", theme.title()),
            Span::raw(last_success),
        ]));
    }

    text_lines.push(Line::from(vec![
        Span::styled("Active Jobs:  ", theme.title()),
        Span::raw(instance.job_count.to_string()),
    ]));
    text_lines.push(Line::from(""));

    // Show error if present
    if let Some(ref error) = instance.error {
        text_lines.push(Line::from(vec![
            Span::styled("Error: ", theme.error()),
            Span::styled(error, theme.error()),
        ]));
        text_lines.push(Line::from(""));
    }

    // Resource table
    if !instance.resources.is_empty() {
        text_lines.push(Line::from(vec![Span::styled("Resources:", theme.title())]));
        text_lines.push(Line::from(""));

        for resource in &instance.resources {
            let status_color = theme.status_color(&resource.status);
            let error_indicator = resource.error.as_ref().map(|_| " (error)").unwrap_or("");

            text_lines.push(Line::from(vec![
                Span::raw(format!("  {:<20} ", resource.resource_type)),
                Span::styled(format!("{:>6}", resource.count), theme.title()),
                Span::raw("  "),
                Span::styled(
                    format!("{}{}", resource.status, error_indicator),
                    Style::default().fg(status_color),
                ),
            ]));
        }
    }

    let text = Text::from(text_lines);
    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Instance Details")
            .border_style(theme.border())
            .title_style(theme.title()),
    );

    f.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{InstanceOverview, MultiInstanceOverviewData, OverviewResource};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_test_multi_instance_data() -> MultiInstanceOverviewData {
        use crate::action::InstanceStatus;
        MultiInstanceOverviewData {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            instances: vec![
                InstanceOverview {
                    profile_name: "prod".to_string(),
                    base_url: "https://splunk.prod.example.com".to_string(),
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
                    ],
                    error: None,
                    health_status: "green".to_string(),
                    job_count: 5,
                    status: InstanceStatus::Healthy,
                    last_success_at: Some("2024-01-01T00:00:00Z".to_string()),
                },
                InstanceOverview {
                    profile_name: "dev".to_string(),
                    base_url: "https://splunk.dev.example.com".to_string(),
                    resources: vec![OverviewResource {
                        resource_type: "indexes".to_string(),
                        count: 10,
                        status: "ok".to_string(),
                        error: None,
                    }],
                    error: None,
                    health_status: "green".to_string(),
                    job_count: 1,
                    status: InstanceStatus::Healthy,
                    last_success_at: Some("2024-01-01T00:00:00Z".to_string()),
                },
            ],
        }
    }

    #[test]
    fn test_render_multi_instance_with_data() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let data = create_test_multi_instance_data();
        let theme = Theme::default();

        terminal
            .draw(|f| {
                render_multi_instance(
                    f,
                    f.area(),
                    MultiInstanceRenderConfig {
                        loading: false,
                        data: Some(&data),
                        selected_index: 0,
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
        assert!(content.contains("Multi-Instance") || content.contains("Instances"));
        assert!(content.contains("prod"));
        assert!(content.contains("dev"));
    }

    #[test]
    fn test_render_multi_instance_loading() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|f| {
                render_multi_instance(
                    f,
                    f.area(),
                    MultiInstanceRenderConfig {
                        loading: true,
                        data: None,
                        selected_index: 0,
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
        assert!(content.contains("Loading"));
    }

    #[test]
    fn test_render_multi_instance_empty() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|f| {
                render_multi_instance(
                    f,
                    f.area(),
                    MultiInstanceRenderConfig {
                        loading: false,
                        data: None,
                        selected_index: 0,
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
        assert!(content.contains("No multi-instance data"));
    }

    #[test]
    fn test_health_status_color() {
        let theme = Theme::default();

        assert_eq!(theme.status_color("green"), theme.success);
        assert_eq!(theme.status_color("healthy"), theme.success);
        assert_eq!(theme.status_color("yellow"), theme.warning);
        assert_eq!(theme.status_color("red"), theme.error);
        assert_eq!(theme.status_color("error"), theme.error);
        assert_eq!(theme.status_color("unknown"), theme.text);
    }

    #[test]
    fn test_resource_status_color() {
        let theme = Theme::default();

        assert_eq!(theme.status_color("ok"), theme.success);
        assert_eq!(theme.status_color("warning"), theme.warning);
        assert_eq!(theme.status_color("error"), theme.error);
    }
}
