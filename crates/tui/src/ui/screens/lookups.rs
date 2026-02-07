//! Lookups screen for the TUI.
//!
//! Responsibilities:
//! - Render the lookup tables list with file metadata
//! - Display loading state and empty state
//!
//! Does NOT handle:
//! - Data fetching (handled by side effects)
//! - User input (handled by input handlers)

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use splunk_client::models::LookupTable;

use splunk_config::Theme;

use crate::ui::theme::spinner_char;

/// Configuration for rendering the lookups screen.
pub struct LookupsRenderConfig<'a> {
    /// Whether data is currently loading.
    pub loading: bool,
    /// The list of lookup tables to display.
    pub lookups: Option<&'a [LookupTable]>,
    /// The table state for selection.
    pub lookups_state: &'a mut TableState,
    /// The theme to use for styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the lookups screen.
pub fn render_lookups(f: &mut Frame, area: Rect, config: LookupsRenderConfig) {
    let LookupsRenderConfig {
        loading,
        lookups,
        lookups_state,
        theme,
        spinner_frame,
    } = config;

    if loading && lookups.is_none() {
        let spinner = spinner_char(spinner_frame);
        let loading_widget =
            ratatui::widgets::Paragraph::new(format!("{} Loading lookup tables...", spinner))
                .block(Block::default().borders(Borders::ALL).title("Lookups"))
                .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let lookups = match lookups {
        Some(l) => l,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No lookup tables loaded. Press 'r' to refresh.")
                    .block(Block::default().borders(Borders::ALL).title("Lookups"))
                    .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    if lookups.is_empty() {
        let empty = ratatui::widgets::Paragraph::new("No lookup tables found.")
            .block(Block::default().borders(Borders::ALL).title("Lookups"))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    // Build table rows
    let header_cells = ["Name", "Filename", "Owner", "App", "Sharing", "Size"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = lookups
        .iter()
        .map(|lookup| {
            let size_str = humanize_size(lookup.size);
            let cells = vec![
                Cell::from(lookup.name.clone()),
                Cell::from(lookup.filename.clone()),
                Cell::from(lookup.owner.clone()),
                Cell::from(lookup.app.clone()),
                Cell::from(lookup.sharing.clone()),
                Cell::from(size_str),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(10),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Lookups"))
    .row_highlight_style(Style::default().bg(theme.accent).fg(theme.background))
    .highlight_symbol("> ");

    f.render_stateful_widget(table, area, lookups_state);
}

/// Convert size in bytes to human-readable string.
fn humanize_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if size == 0 {
        return "0 B".to_string();
    }
    let size_f = size as f64;
    let exp = (size_f.log2() / 1024_f64.log2()).min(UNITS.len() as f64 - 1.0) as usize;
    let value = size_f / 1024_f64.powi(exp as i32);
    format!("{:.1} {}", value, UNITS[exp])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_humanize_size() {
        assert_eq!(humanize_size(0), "0 B");
        assert_eq!(humanize_size(512), "512.0 B");
        assert_eq!(humanize_size(1024), "1.0 KB");
        assert_eq!(humanize_size(1536), "1.5 KB");
        assert_eq!(humanize_size(1024 * 1024), "1.0 MB");
        assert_eq!(humanize_size(1024 * 1024 * 1024), "1.0 GB");
    }
}
