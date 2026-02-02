//! Search macros screen rendering.
//!
//! Responsibilities:
//! - Render the macros list with a split view (list + preview).
//! - Handle selection highlighting and scrolling.
//!
//! Non-responsibilities:
//! - Does not handle input (see app/input/macros.rs).
//! - Does not fetch data (see runtime/side_effects/macros.rs).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::state::CurrentScreen;
use splunk_config::Theme;

/// Render the macros screen with a split view (list on top, preview on bottom).
pub fn render_macros_screen(
    f: &mut Frame,
    area: Rect,
    macros: Option<&[splunk_client::Macro]>,
    list_state: &mut ratatui::widgets::ListState,
    loading: bool,
    theme: &Theme,
) {
    // Split area into list (top 50%) and preview (bottom 50%)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let list_area = chunks[0];
    let preview_area = chunks[1];

    // Render the macros list
    render_macros_list(f, list_area, macros, list_state, loading, theme);

    // Render the macro preview
    render_macro_preview(f, preview_area, macros, list_state, theme);
}

fn render_macros_list(
    f: &mut Frame,
    area: Rect,
    macros: Option<&[splunk_client::Macro]>,
    list_state: &mut ratatui::widgets::ListState,
    loading: bool,
    theme: &Theme,
) {
    let block = Block::default()
        .title(" Search Macros ")
        .borders(Borders::ALL)
        .border_style(theme.border);

    if loading {
        let loading_text = Paragraph::new("Loading macros...")
            .block(block)
            .style(theme.text);
        f.render_widget(loading_text, area);
        return;
    }

    match macros {
        None => {
            let empty_text = Paragraph::new("Press 'r' to load macros")
                .block(block)
                .style(theme.text);
            f.render_widget(empty_text, area);
        }
        Some([]) => {
            let empty_text = Paragraph::new("No macros found")
                .block(block)
                .style(theme.text);
            f.render_widget(empty_text, area);
        }
        Some(macros_list) => {
            let items: Vec<ListItem> = macros_list
                .iter()
                .map(|m| {
                    let name = &m.name;
                    let disabled_marker = if m.disabled { " [DISABLED]" } else { "" };
                    let eval_marker = if m.iseval { " (eval)" } else { "" };
                    let content = format!("{}{}{}", name, eval_marker, disabled_marker);

                    let style = if m.disabled {
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC)
                    } else {
                        Style::default()
                    };

                    ListItem::new(content).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(block)
                .highlight_style(
                    Style::default()
                        .bg(theme.highlight_bg)
                        .fg(theme.highlight_fg)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            f.render_stateful_widget(list, area, list_state);
        }
    }
}

fn render_macro_preview(
    f: &mut Frame,
    area: Rect,
    macros: Option<&[splunk_client::Macro]>,
    list_state: &ratatui::widgets::ListState,
    theme: &Theme,
) {
    let block = Block::default()
        .title(" Macro Definition ")
        .borders(Borders::ALL)
        .border_style(theme.border);

    let content = match (macros, list_state.selected()) {
        (Some(macros_list), Some(selected)) if selected < macros_list.len() => {
            let macro_item = &macros_list[selected];
            let mut lines = vec![];

            // Name
            lines.push(Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&macro_item.name),
            ]));

            // Args if present
            if let Some(ref args) = macro_item.args {
                lines.push(Line::from(vec![
                    Span::styled("Arguments: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(args),
                ]));
            }

            // Description if present
            if let Some(ref desc) = macro_item.description {
                lines.push(Line::from(vec![
                    Span::styled(
                        "Description: ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(desc),
                ]));
            }

            // Flags
            let mut flags = vec![];
            if macro_item.disabled {
                flags.push("disabled");
            }
            if macro_item.iseval {
                flags.push("eval");
            }
            if !flags.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("Flags: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(flags.join(", ")),
                ]));
            }

            // Validation if present
            if let Some(ref validation) = macro_item.validation {
                lines.push(Line::from(vec![
                    Span::styled(
                        "Validation: ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(validation),
                ]));
            }

            // Error message if present
            if let Some(ref errormsg) = macro_item.errormsg {
                lines.push(Line::from(vec![
                    Span::styled(
                        "Error Message: ",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(errormsg),
                ]));
            }

            // Separator
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Definition:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));

            // Definition
            for line in macro_item.definition.lines() {
                lines.push(Line::from(line.to_string()));
            }

            Text::from(lines)
        }
        _ => Text::from("Select a macro to view its definition"),
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .style(theme.text)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Get the title for the macros screen.
pub fn title() -> &'static str {
    "Macros"
}

/// Get the help text for the macros screen.
pub fn help_text() -> &'static str {
    "r: Refresh | e: Edit | n: New | d: Delete | Ctrl+C: Copy | Enter: Details | q: Quit"
}

/// Get the current screen variant for macros.
pub fn current_screen() -> CurrentScreen {
    CurrentScreen::Macros
}
