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
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use splunk_client::{format_bytes, models::LookupTable};

use splunk_config::Theme;

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};

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
        render_loading_state(
            f,
            area,
            "Lookups",
            "Loading lookup tables...",
            spinner_frame,
            theme,
        );
        return;
    }

    let lookups = match lookups {
        Some(l) => l,
        None => {
            render_empty_state(f, area, "Lookups", "lookup tables");
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
        .map(|h| Cell::from(*h).style(theme.table_header()));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = lookups
        .iter()
        .map(|lookup| {
            let size_str = format_bytes(lookup.size);
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
    .row_highlight_style(theme.highlight())
    .highlight_symbol("> ");

    f.render_stateful_widget(table, area, lookups_state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512.0 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }
}
