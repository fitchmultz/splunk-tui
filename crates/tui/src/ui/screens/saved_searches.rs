//! Saved searches screen rendering.
//!
//! Renders the list of Splunk saved searches.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use splunk_client::models::SavedSearch;

use crate::ui::syntax::highlight_spl;
use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};
use splunk_config::Theme;

/// Configuration for rendering the saved searches screen.
pub struct SavedSearchesRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of saved searches to display
    pub saved_searches: Option<&'a [SavedSearch]>,
    /// The current list selection state
    pub state: &'a mut ListState,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the saved searches screen.
pub fn render_saved_searches(f: &mut Frame, area: Rect, config: SavedSearchesRenderConfig) {
    let SavedSearchesRenderConfig {
        loading,
        saved_searches,
        state,
        theme,
        spinner_frame,
    } = config;

    if loading && saved_searches.is_none() {
        render_loading_state(
            f,
            area,
            "Saved Searches",
            "Loading saved searches...",
            spinner_frame,
            theme,
        );
        return;
    }

    let searches = match saved_searches {
        Some(s) => s,
        None => {
            render_empty_state(f, area, "Saved Searches", "saved searches");
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
                theme.disabled()
            } else {
                theme.text()
            };
            ListItem::new(s.name.clone()).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Saved Searches")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .highlight_style(theme.highlight());
    f.render_stateful_widget(list, chunks[0], state);

    // Preview area
    let selected_search = state.selected().and_then(|i| searches.get(i));
    let preview_content = if let Some(s) = selected_search {
        let mut details = vec![Line::from(vec![Span::styled(
            "Search Query:",
            theme.title(),
        )])];
        details.extend(highlight_spl(&s.search, theme).lines);

        if let Some(desc) = &s.description {
            details.push(Line::from(""));
            details.push(Line::from(vec![
                Span::styled("Description: ", theme.title()),
                Span::raw(desc),
            ]));
        }

        details.push(Line::from(""));
        details.push(Line::from(Span::styled(
            "Press Enter to run this search",
            theme.title(),
        )));

        details
    } else {
        vec![Line::from("Select a saved search to see details")]
    };

    let preview = Paragraph::new(preview_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Preview")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(preview, chunks[1]);
}
