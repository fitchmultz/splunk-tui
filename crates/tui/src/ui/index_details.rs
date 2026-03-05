//! Index details popup rendering with structured display.

use ratatui::{
    Frame,
    layout::{Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use crate::app::App;
use splunk_client::models::Index;
use splunk_config::Theme;

/// Render the index details popup.
///
/// Displays all Index fields in a structured key/value format with scrolling support.
pub fn render_index_details(f: &mut Frame, app: &App, theme: &Theme) {
    let area = f.area();

    // Calculate popup dimensions - ensure we don't exceed the available area
    let popup_width = 80.min(area.width.saturating_sub(4));
    let popup_height = 25.min(area.height.saturating_sub(4));

    // Calculate centered position
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    // Get the selected index
    let index = match get_selected_index(app) {
        Some(idx) => idx,
        None => {
            render_no_index_message(f, popup_area, theme);
            return;
        }
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(&index.name, Style::default().fg(theme.accent)),
        ]),
        Line::default(),
    ];

    // Basic stats
    lines.push(Line::from(vec![
        Span::styled(
            "Total Event Count: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", index.total_event_count)),
    ]));
    lines.push(Line::default());

    lines.push(Line::from(vec![
        Span::styled(
            "Current DB Size: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{} MB", index.current_db_size_mb)),
    ]));
    lines.push(Line::default());

    // Size limits
    if let Some(max_size) = index.max_total_data_size_mb {
        lines.push(Line::from(vec![
            Span::styled(
                "Max Total Data Size: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{} MB", max_size)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "Max Total Data Size: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("Not set", Style::default().fg(theme.text_dim)),
        ]));
    }
    lines.push(Line::default());

    // Bucket configuration
    if let Some(max_warm) = index.max_warm_db_count {
        lines.push(Line::from(vec![
            Span::styled(
                "Max Warm DB Count: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}", max_warm)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "Max Warm DB Count: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("Not set", Style::default().fg(theme.text_dim)),
        ]));
    }
    lines.push(Line::default());

    if let Some(max_hot) = &index.max_hot_buckets {
        lines.push(Line::from(vec![
            Span::styled(
                "Max Hot Buckets: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(max_hot),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "Max Hot Buckets: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("Not set", Style::default().fg(theme.text_dim)),
        ]));
    }
    lines.push(Line::default());

    // Retention settings
    if let Some(frozen_time) = index.frozen_time_period_in_secs {
        let days = frozen_time / 86400;
        lines.push(Line::from(vec![
            Span::styled(
                "Frozen Time Period: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{} seconds ({} days)", frozen_time, days)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "Frozen Time Period: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("Not set", Style::default().fg(theme.text_dim)),
        ]));
    }
    lines.push(Line::default());

    // Storage paths
    if let Some(home_path) = &index.home_path {
        lines.push(Line::from(vec![
            Span::styled("Home Path: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(home_path),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Home Path: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("Not set", Style::default().fg(theme.text_dim)),
        ]));
    }
    lines.push(Line::default());

    if let Some(cold_path) = &index.cold_db_path {
        lines.push(Line::from(vec![
            Span::styled(
                "Cold DB Path: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(cold_path),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "Cold DB Path: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("Not set", Style::default().fg(theme.text_dim)),
        ]));
    }
    lines.push(Line::default());

    if let Some(thawed_path) = &index.thawed_path {
        lines.push(Line::from(vec![
            Span::styled(
                "Thawed Path: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(thawed_path),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "Thawed Path: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("Not set", Style::default().fg(theme.text_dim)),
        ]));
    }
    lines.push(Line::default());

    if let Some(frozen_dir) = &index.cold_to_frozen_dir {
        lines.push(Line::from(vec![
            Span::styled(
                "Cold to Frozen Dir: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(frozen_dir),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "Cold to Frozen Dir: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("Not set", Style::default().fg(theme.text_dim)),
        ]));
    }
    lines.push(Line::default());

    // Primary index flag
    if let Some(primary) = index.primary_index {
        lines.push(Line::from(vec![
            Span::styled(
                "Primary Index: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(if primary { "Yes" } else { "No" }),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "Primary Index: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("Unknown", Style::default().fg(theme.text_dim)),
        ]));
    }

    let paragraph = Paragraph::new(lines.clone())
        .block(
            Block::default()
                .title("Index Details")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.index_details_scroll_offset as u16, 0));

    f.render_widget(paragraph, popup_area);

    let content_height = lines.len();
    let visible_lines = popup_height.saturating_sub(2) as usize;

    if content_height > visible_lines {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::new(content_height.saturating_sub(1))
            .position(app.index_details_scroll_offset);
        f.render_stateful_widget(
            scrollbar,
            popup_area.inner(Margin::new(0, 1)),
            &mut scrollbar_state,
        );
    }
}

/// Get the currently selected index from the app state.
fn get_selected_index(app: &App) -> Option<&Index> {
    app.indexes
        .as_ref()
        .and_then(|indexes| app.indexes_state.selected().and_then(|i| indexes.get(i)))
}

/// Render a message when no index is selected.
fn render_no_index_message(f: &mut Frame, area: Rect, theme: &Theme) {
    let lines = vec![
        Line::from(Span::styled(
            "No index selected",
            Style::default().fg(theme.error),
        )),
        Line::default(),
        Line::from(Span::raw("Press Esc or q to close")),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title("Index Details")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        )
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}
