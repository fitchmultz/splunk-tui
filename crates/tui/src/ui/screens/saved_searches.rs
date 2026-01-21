//! Saved searches screen rendering.
//!
//! Renders the list of Splunk saved searches.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use splunk_client::models::SavedSearch;

/// Configuration for rendering the saved searches screen.
pub struct SavedSearchesRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of saved searches to display
    pub saved_searches: Option<&'a [SavedSearch]>,
    /// The current list selection state
    pub state: &'a mut ListState,
}

/// Render the saved searches screen.
pub fn render_saved_searches(f: &mut Frame, area: Rect, config: SavedSearchesRenderConfig) {
    let SavedSearchesRenderConfig {
        loading,
        saved_searches,
        state,
    } = config;

    if loading && saved_searches.is_none() {
        let loading_widget = Paragraph::new("Loading saved searches...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Saved Searches"),
            )
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let searches = match saved_searches {
        Some(s) => s,
        None => {
            let placeholder = Paragraph::new("No saved searches loaded. Press 'r' to refresh.")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Saved Searches"),
                )
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

    let items: Vec<ListItem> = searches
        .iter()
        .map(|s| {
            let style = if s.disabled {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(s.name.clone()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Saved Searches"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(list, chunks[0], state);

    // Preview area
    let selected_search = state.selected().and_then(|i| searches.get(i));
    let preview_content = if let Some(s) = selected_search {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&s.name),
            ]),
            Line::from(vec![
                Span::styled("Disabled: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if s.disabled { "Yes" } else { "No" },
                    if s.disabled {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    },
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Search Query:",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(&s.search, Style::default().fg(Color::Cyan))),
        ];

        if let Some(desc) = &s.description {
            lines.insert(
                2,
                Line::from(vec![
                    Span::styled(
                        "Description: ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(desc),
                ]),
            );
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to run this search",
            Style::default().fg(Color::Yellow),
        )));

        lines
    } else {
        vec![Line::from("Select a saved search to see details")]
    };

    let preview = Paragraph::new(preview_content)
        .block(Block::default().borders(Borders::ALL).title("Preview"))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(preview, chunks[1]);
}
