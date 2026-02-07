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
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use splunk_config::Theme;

use crate::action::MultiInstanceOverviewData;

use crate::ui::theme::spinner_char;

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
        let spinner = spinner_char(spinner_frame);
        let loading_widget =
            Paragraph::new(format!("{} Loading multi-instance dashboard...", spinner))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Multi-Instance"),
                )
                .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let overview_data = match data {
        Some(d) => d,
        None => {
            let placeholder =
                Paragraph::new("No multi-instance data loaded. Press 'r' to refresh.")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Multi-Instance"),
                    )
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
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
    let header = Row::new(vec!["Instance", "Health", "Jobs"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .height(1);

    let rows: Vec<Row> = data
        .instances
        .iter()
        .enumerate()
        .map(|(idx, instance)| {
            let is_selected = idx == selected_index;
            let health_color = health_status_color(&instance.health_status, theme);
            let row_style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let health_indicator = if instance.error.is_some() {
                "✗ Error"
            } else {
                &instance.health_status
            };

            Row::new(vec![
                Cell::from(instance.profile_name.clone()),
                Cell::from(Span::styled(
                    health_indicator.to_string(),
                    Style::default().fg(health_color),
                )),
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
            .border_style(Style::default().fg(theme.border))
            .title_style(Style::default().fg(theme.title)),
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

    // Build the content
    let mut text_lines: Vec<Line> = vec![
        // Instance header
        Line::from(vec![
            Span::styled("Instance: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&instance.profile_name),
        ]),
        Line::from(vec![
            Span::styled("URL: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&instance.base_url),
        ]),
        Line::from(vec![
            Span::styled("Health: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                &instance.health_status,
                Style::default().fg(health_status_color(&instance.health_status, theme)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Active Jobs: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(instance.job_count.to_string()),
        ]),
        Line::from(""),
    ];

    // Show error if present
    if let Some(ref error) = instance.error {
        text_lines.push(Line::from(vec![
            Span::styled(
                "Error: ",
                Style::default()
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(error, Style::default().fg(theme.error)),
        ]));
        text_lines.push(Line::from(""));
    }

    // Resource table
    if !instance.resources.is_empty() {
        text_lines.push(Line::from(vec![Span::styled(
            "Resources:",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        text_lines.push(Line::from(""));

        for resource in &instance.resources {
            let status_color = resource_status_color(&resource.status, theme);
            let error_indicator = resource.error.as_ref().map(|_| " (error)").unwrap_or("");

            text_lines.push(Line::from(vec![
                Span::raw(format!("  {:<20} ", resource.resource_type)),
                Span::styled(
                    format!("{:>6}", resource.count),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
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
            .border_style(Style::default().fg(theme.border))
            .title_style(Style::default().fg(theme.title)),
    );

    f.render_widget(paragraph, area);
}

/// Get color for health status.
fn health_status_color(status: &str, theme: &Theme) -> ratatui::style::Color {
    match status.to_lowercase().as_str() {
        "green" | "healthy" | "ok" => theme.success,
        "yellow" | "degraded" | "warning" => theme.warning,
        "red" | "unhealthy" | "error" | "critical" => theme.error,
        _ => theme.text,
    }
}

/// Get color for resource status.
fn resource_status_color(status: &str, theme: &Theme) -> ratatui::style::Color {
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
    use crate::action::{InstanceOverview, MultiInstanceOverviewData, OverviewResource};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_test_multi_instance_data() -> MultiInstanceOverviewData {
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

        assert_eq!(health_status_color("green", &theme), theme.success);
        assert_eq!(health_status_color("healthy", &theme), theme.success);
        assert_eq!(health_status_color("yellow", &theme), theme.warning);
        assert_eq!(health_status_color("red", &theme), theme.error);
        assert_eq!(health_status_color("error", &theme), theme.error);
        assert_eq!(health_status_color("unknown", &theme), theme.text);
    }

    #[test]
    fn test_resource_status_color() {
        let theme = Theme::default();

        assert_eq!(resource_status_color("ok", &theme), theme.success);
        assert_eq!(resource_status_color("warning", &theme), theme.warning);
        assert_eq!(resource_status_color("error", &theme), theme.error);
    }
}
