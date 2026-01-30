//! Fired alerts screen rendering.
//!
//! Renders the list of Splunk fired alerts.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use splunk_client::models::FiredAlert;

use splunk_config::Theme;

/// Configuration for rendering the fired alerts screen.
pub struct FiredAlertsRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of fired alerts to display
    pub fired_alerts: Option<&'a [FiredAlert]>,
    /// The current list selection state
    pub state: &'a mut ListState,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
}

/// Render the fired alerts screen.
pub fn render_fired_alerts(f: &mut Frame, area: Rect, config: FiredAlertsRenderConfig) {
    let FiredAlertsRenderConfig {
        loading,
        fired_alerts,
        state,
        theme,
    } = config;

    if loading && fired_alerts.is_none() {
        let loading_widget = Paragraph::new("Loading fired alerts...")
            .block(Block::default().borders(Borders::ALL).title("Fired Alerts"))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let alerts = match fired_alerts {
        Some(a) => a,
        None => {
            let placeholder = Paragraph::new("No fired alerts loaded. Press 'r' to refresh.")
                .block(Block::default().borders(Borders::ALL).title("Fired Alerts"))
                .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    // Create a split layout for list and preview
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let items: Vec<ListItem> = alerts
        .iter()
        .map(|a| {
            let style = Style::default().fg(theme.text);
            let name = &a.name;
            let savedsearch = a.savedsearch_name.as_deref().unwrap_or("-");
            let label = format!("{} ({})", name, savedsearch);
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Fired Alerts")
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.title)),
        )
        .highlight_style(
            Style::default()
                .fg(theme.highlight_fg)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(list, chunks[0], state);

    // Preview area
    let selected_alert = state.selected().and_then(|i| alerts.get(i));
    let preview_content = if let Some(a) = selected_alert {
        let mut details = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&a.name),
            ]),
            Line::from(vec![
                Span::styled(
                    "Saved Search: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(a.savedsearch_name.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled("Severity: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(a.severity.as_deref().unwrap_or("Medium")),
            ]),
            Line::from(vec![
                Span::styled(
                    "Trigger Time: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(a.trigger_time_rendered.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled("SID: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(a.sid.as_deref().unwrap_or("-")),
            ]),
        ];

        if let Some(ref actions) = a.actions {
            details.push(Line::from(""));
            details.push(Line::from(vec![
                Span::styled("Actions: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(actions),
            ]));
        }

        details
    } else {
        vec![Line::from("Select a fired alert to see details")]
    };

    let preview = Paragraph::new(preview_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Details")
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.title)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(preview, chunks[1]);
}
