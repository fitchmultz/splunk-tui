//! License screen rendering.
//!
//! Renders comprehensive Splunk license information including usage,
//! license pools, and license stacks.

use crate::action::LicenseData;
use crate::ui::widgets::{render_empty_state, render_loading_state};
use crate::utils::format_bytes;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use splunk_config::Theme;

use crate::ui::theme::ThemeExt;

/// Configuration for rendering the license screen.
pub struct LicenseRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The license information to display
    pub license_info: Option<&'a LicenseData>,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the license screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_license(f: &mut Frame, area: Rect, config: LicenseRenderConfig) {
    let LicenseRenderConfig {
        loading,
        license_info,
        theme,
        spinner_frame,
    } = config;

    if loading && license_info.is_none() {
        render_loading_state(
            f,
            area,
            "License",
            "Loading license info...",
            spinner_frame,
            theme,
        );
        return;
    }

    let info = match license_info {
        Some(i) => i,
        None => {
            render_empty_state(f, area, "License", "license info");
            return;
        }
    };

    // Create layout with three sections: usage, pools, stacks
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30), // Usage section
            Constraint::Percentage(35), // Pools section
            Constraint::Percentage(35), // Stacks section
        ])
        .split(area);

    render_usage_section(f, chunks[0], info, theme);
    render_pools_section(f, chunks[1], info, theme);
    render_stacks_section(f, chunks[2], info, theme);
}

/// Render the license usage section.
fn render_usage_section(f: &mut Frame, area: Rect, info: &LicenseData, theme: &Theme) {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if info.usage.is_empty() {
        lines.push(Line::from("No license usage data available."));
    } else {
        for (i, usage) in info.usage.iter().enumerate() {
            if i > 0 {
                lines.push(Line::from(""));
            }

            let used_bytes = usage.effective_used_bytes();
            let percentage = if usage.quota > 0 {
                (used_bytes as f64 / usage.quota as f64) * 100.0
            } else {
                0.0
            };

            let (pct_text, pct_color) = percentage_span(percentage, theme);

            lines.push(Line::from(vec![
                Span::styled(format!("License {}: ", i + 1), theme.title()),
                Span::styled(pct_text, Style::default().fg(pct_color)),
            ]));

            if !usage.name.is_empty() {
                lines.push(Line::from(format!("  Name: {}", usage.name)));
            }

            lines.push(Line::from(format!(
                "  Used: {} / {}",
                format_bytes(used_bytes),
                format_bytes(usage.quota)
            )));

            if let Some(ref stack_id) = usage.stack_id {
                lines.push(Line::from(format!("  Stack: {}", stack_id)));
            }

            // Show per-slave breakdown if available
            if let Some(slaves) = usage.slaves_breakdown() {
                lines.push(Line::from("  Per-Slave Usage:"));
                for (slave, bytes) in slaves.iter().take(5) {
                    lines.push(Line::from(format!(
                        "    {}: {}",
                        slave,
                        format_bytes(*bytes)
                    )));
                }
                if slaves.len() > 5 {
                    lines.push(Line::from(format!("    ... and {} more", slaves.len() - 5)));
                }
            }
        }
    }

    let usage_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("License Usage")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .alignment(Alignment::Left);

    f.render_widget(usage_widget, area);
}

/// Render the license pools table.
fn render_pools_section(f: &mut Frame, area: Rect, info: &LicenseData, theme: &Theme) {
    let header = Row::new(vec![
        Cell::from("Name").style(theme.table_header()),
        Cell::from("Quota").style(theme.table_header()),
        Cell::from("Used").style(theme.table_header()),
        Cell::from("Stack ID").style(theme.table_header()),
    ])
    .height(1);

    let rows: Vec<Row> = if info.pools.is_empty() {
        vec![Row::new(vec![
            Cell::from("No pools available"),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ])]
    } else {
        info.pools
            .iter()
            .map(|pool| {
                Row::new(vec![
                    Cell::from(pool.name.clone()),
                    Cell::from(pool.quota.clone()),
                    Cell::from(format_bytes(pool.used_bytes)),
                    Cell::from(pool.stack_id.clone()),
                ])
            })
            .collect()
    };

    let widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(20),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("License Pools")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .column_spacing(1);

    f.render_widget(table, area);
}

/// Render the license stacks table.
fn render_stacks_section(f: &mut Frame, area: Rect, info: &LicenseData, theme: &Theme) {
    let header = Row::new(vec![
        Cell::from("Name").style(theme.table_header()),
        Cell::from("Type").style(theme.table_header()),
        Cell::from("Label").style(theme.table_header()),
        Cell::from("Quota").style(theme.table_header()),
    ])
    .height(1);

    let rows: Vec<Row> = if info.stacks.is_empty() {
        vec![Row::new(vec![
            Cell::from("No stacks available"),
            Cell::from(""),
            Cell::from(""),
            Cell::from(""),
        ])]
    } else {
        info.stacks
            .iter()
            .map(|stack| {
                Row::new(vec![
                    Cell::from(stack.name.clone()),
                    Cell::from(stack.type_name.clone()),
                    Cell::from(stack.label.clone()),
                    Cell::from(format_bytes(stack.quota)),
                ])
            })
            .collect()
    };

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(20),
        Constraint::Percentage(30),
        Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("License Stacks")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .column_spacing(1);

    f.render_widget(table, area);
}

/// Format license usage percentage and choose a semantic color.
fn percentage_span(percentage: f64, theme: &Theme) -> (String, ratatui::style::Color) {
    let color = if percentage < 70.0 {
        theme.success
    } else if percentage < 90.0 {
        theme.warning
    } else {
        theme.error
    };
    (format!("{:.1}%", percentage), color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use splunk_client::models::{LicensePool, LicenseStack, LicenseUsage};

    #[test]
    fn test_percentage_span() {
        let theme = Theme::default();

        // Below 70% should be success color
        let (text, color) = percentage_span(50.0, &theme);
        assert_eq!(text, "50.0%");
        assert_eq!(color, theme.success);

        // 70-90% should be warning color
        let (text, color) = percentage_span(80.0, &theme);
        assert_eq!(text, "80.0%");
        assert_eq!(color, theme.warning);

        // Above 90% should be error color
        let (text, color) = percentage_span(95.0, &theme);
        assert_eq!(text, "95.0%");
        assert_eq!(color, theme.error);
    }

    #[test]
    fn test_license_data_empty() {
        let data = LicenseData {
            usage: vec![],
            pools: vec![],
            stacks: vec![],
        };

        assert!(data.usage.is_empty());
        assert!(data.pools.is_empty());
        assert!(data.stacks.is_empty());
    }

    #[test]
    fn test_license_data_with_values() {
        let data = LicenseData {
            usage: vec![LicenseUsage {
                name: "test_license".to_string(),
                quota: 1024 * 1024 * 1024,           // 1 GB
                used_bytes: Some(512 * 1024 * 1024), // 512 MB
                slaves_usage_bytes: None,
                stack_id: Some("stack1".to_string()),
            }],
            pools: vec![LicensePool {
                name: "pool1".to_string(),
                quota: "1GB".to_string(),
                used_bytes: 512 * 1024 * 1024,
                stack_id: "stack1".to_string(),
                description: Some("Test pool".to_string()),
            }],
            stacks: vec![LicenseStack {
                name: "stack1".to_string(),
                quota: 1024 * 1024 * 1024,
                type_name: "enterprise".to_string(),
                label: "Enterprise Stack".to_string(),
            }],
        };

        assert_eq!(data.usage.len(), 1);
        assert_eq!(data.pools.len(), 1);
        assert_eq!(data.stacks.len(), 1);

        assert_eq!(data.usage[0].name, "test_license");
        assert_eq!(data.pools[0].name, "pool1");
        assert_eq!(data.stacks[0].name, "stack1");
    }
}
