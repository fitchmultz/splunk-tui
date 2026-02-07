//! Configs screen rendering.
//!
//! Renders the configuration file browser and viewer for Splunk config files
//! (props.conf, transforms.conf, inputs.conf, etc.).

use crate::app::input::components::SingleLineInput;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
};
use splunk_client::models::{ConfigFile, ConfigStanza};
use splunk_config::Theme;

use crate::ui::theme::spinner_char;

/// View mode for the configs screen.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ConfigViewMode {
    /// List of config files
    #[default]
    FileList,
    /// Stanzas within selected file
    StanzaList,
    /// Full stanza content view
    StanzaDetail,
}

/// Configuration for rendering the configs screen.
pub struct ConfigsRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of config files to display
    pub config_files: Option<&'a [ConfigFile]>,
    /// The currently selected config file
    pub selected_file: Option<&'a str>,
    /// The list of stanzas for the selected file
    pub stanzas: Option<&'a [ConfigStanza]>,
    /// The currently selected stanza (for detail view)
    pub selected_stanza: Option<&'a ConfigStanza>,
    /// Current view mode
    pub view_mode: ConfigViewMode,
    /// The current table selection state for config files
    pub files_state: &'a mut TableState,
    /// The current table selection state for stanzas
    pub stanzas_state: &'a mut TableState,
    /// Theme for consistent styling
    pub theme: &'a Theme,
    /// Whether search mode is active
    pub is_searching: bool,
    /// Current search query (single-line input component)
    pub search_query: &'a SingleLineInput,
    /// Filtered indices when searching
    pub filtered_indices: &'a [usize],
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the configs screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_configs(f: &mut Frame, area: Rect, config: ConfigsRenderConfig) {
    match config.view_mode {
        ConfigViewMode::FileList => render_file_list(f, area, config),
        ConfigViewMode::StanzaList => render_stanza_list(f, area, config),
        ConfigViewMode::StanzaDetail => render_stanza_detail(f, area, config),
    }
}

/// Render the config files list view.
fn render_file_list(f: &mut Frame, area: Rect, config: ConfigsRenderConfig) {
    let ConfigsRenderConfig {
        loading,
        config_files,
        files_state,
        theme,
        spinner_frame,
        ..
    } = config;

    if loading && config_files.is_none() {
        let spinner = spinner_char(spinner_frame);
        let loading_widget = Paragraph::new(format!("{} Loading config files...", spinner))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Configuration Files"),
            )
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let files = match config_files {
        Some(f) => f,
        None => {
            let placeholder = Paragraph::new("No config files loaded. Press 'r' to refresh.")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Configuration Files"),
                )
                .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    if files.is_empty() {
        let placeholder = Paragraph::new("No config files found.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Configuration Files"),
            )
            .alignment(Alignment::Center);
        f.render_widget(placeholder, area);
        return;
    }

    // Header
    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Title").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Description").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    // Rows
    let rows: Vec<Row> = files
        .iter()
        .map(|file| {
            Row::new(vec![
                Cell::from(file.name.as_str()),
                Cell::from(file.title.as_str()),
                Cell::from(file.description.as_deref().unwrap_or("-")),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(50),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Configuration Files (Enter to view stanzas)")
            .border_style(Style::default().fg(theme.border))
            .title_style(Style::default().fg(theme.title)),
    )
    .row_highlight_style(
        Style::default()
            .fg(theme.highlight_fg)
            .bg(theme.highlight_bg)
            .add_modifier(Modifier::BOLD),
    );

    f.render_stateful_widget(table, area, files_state);
}

/// Render the stanzas list view for a selected config file.
fn render_stanza_list(f: &mut Frame, area: Rect, config: ConfigsRenderConfig) {
    let ConfigsRenderConfig {
        loading,
        selected_file,
        stanzas,
        stanzas_state,
        theme,
        is_searching,
        search_query,
        filtered_indices,
        spinner_frame,
        ..
    } = config;

    let title = format!("Stanzas for {}.conf", selected_file.unwrap_or("unknown"));

    if loading && stanzas.is_none() {
        let spinner = spinner_char(spinner_frame);
        let loading_widget = Paragraph::new(format!("{} Loading stanzas...", spinner))
            .block(Block::default().borders(Borders::ALL).title(title))
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let stanzas = match stanzas {
        Some(s) => s,
        None => {
            let placeholder = Paragraph::new("No stanzas loaded. Press 'r' to refresh.")
                .block(Block::default().borders(Borders::ALL).title(title))
                .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    // Split area for search bar if needed
    let (search_area, list_area) = if is_searching || !search_query.is_empty() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area)
    };

    // Render search bar if active
    if let Some(search_area) = search_area {
        let query_value = search_query.value();
        let search_text = if is_searching {
            format!("Search: {}", query_value)
        } else {
            format!("Search: {} (Press / to edit, Esc to clear)", query_value)
        };

        let search_paragraph = Paragraph::new(search_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Search Stanzas")
                    .border_style(Style::default().fg(theme.border))
                    .title_style(Style::default().fg(theme.title)),
            )
            .alignment(Alignment::Left);
        f.render_widget(search_paragraph, search_area);
    }

    // Use filtered stanzas if search is active
    let stanzas_to_render: Vec<&ConfigStanza> = if search_query.is_empty() {
        stanzas.iter().collect()
    } else {
        filtered_indices
            .iter()
            .filter_map(|&i| stanzas.get(i))
            .collect()
    };

    if stanzas_to_render.is_empty() {
        let placeholder_text = if search_query.is_empty() {
            "No stanzas found for this config file."
        } else {
            "No stanzas match the search query."
        };
        let placeholder = Paragraph::new(placeholder_text)
            .block(Block::default().borders(Borders::ALL).title(title))
            .alignment(Alignment::Center);
        f.render_widget(placeholder, list_area);
        return;
    }

    // Header
    let header = Row::new(vec![
        Cell::from("Stanza Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Settings Preview").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    // Rows
    let rows: Vec<Row> = stanzas_to_render
        .iter()
        .map(|stanza| {
            // Show a preview of the first few settings
            let settings_preview: String = stanza
                .settings
                .iter()
                .take(3)
                .map(|(k, v)| format!("{} = {}", k, v))
                .collect::<Vec<_>>()
                .join(", ");
            let preview = if settings_preview.len() > 60 {
                format!("{}...", &settings_preview[..57])
            } else {
                settings_preview
            };
            let preview_display = if preview.is_empty() {
                "(no settings)".to_string()
            } else {
                preview
            };

            Row::new(vec![
                Cell::from(stanza.name.as_str()),
                Cell::from(preview_display),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Percentage(40), Constraint::Percentage(60)],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("{} (Enter for details, h to go back)", title))
            .border_style(Style::default().fg(theme.border))
            .title_style(Style::default().fg(theme.title)),
    )
    .row_highlight_style(
        Style::default()
            .fg(theme.highlight_fg)
            .bg(theme.highlight_bg)
            .add_modifier(Modifier::BOLD),
    );

    f.render_stateful_widget(table, list_area, stanzas_state);

    // Render search popup overlay if in search mode
    if is_searching {
        render_search_popup(f, search_query.value(), theme);
    }
}

/// Render the stanza detail view.
fn render_stanza_detail(f: &mut Frame, area: Rect, config: ConfigsRenderConfig) {
    let ConfigsRenderConfig {
        selected_stanza,
        theme,
        ..
    } = config;

    let stanza = match selected_stanza {
        Some(s) => s,
        None => {
            let placeholder = Paragraph::new("No stanza selected. Press 'h' to go back.")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Stanza Detail"),
                )
                .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    let title = format!("Stanza: [{}]", stanza.name);

    // Build content
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "Config File: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{}.conf", stanza.config_file)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Settings:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ];

    if stanza.settings.is_empty() {
        lines.push(Line::from("  (no settings)"));
    } else {
        for (key, value) in &stanza.settings {
            lines.push(Line::from(vec![
                Span::raw(format!("  {} = ", key)),
                Span::styled(value.to_string(), Style::default().fg(theme.success)),
            ]));
        }
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.title)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Render a help popup for the configs screen.
pub fn render_configs_help(f: &mut Frame, theme: &Theme) {
    let help_text = vec![
        Line::from(Span::styled(
            "Configuration Files Help",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  ↑/↓ or k/j    Navigate up/down"),
        Line::from("  Enter         Select / view details"),
        Line::from("  h             Go back to previous view"),
        Line::from(""),
        Line::from("Actions:"),
        Line::from("  r/F5          Refresh data"),
        Line::from("  y             Copy stanza name to clipboard"),
        Line::from("  /             Search stanzas"),
        Line::from("  ?             Show this help"),
        Line::from(""),
        Line::from("Press any key to close..."),
    ];

    let help_paragraph = Paragraph::new(Text::from(help_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help")
                .border_style(Style::default().fg(theme.border)),
        )
        .alignment(Alignment::Left);

    // Calculate centered area
    let area = f.area();
    let popup_area = Rect {
        x: area.x + area.width / 4,
        y: area.y + area.height / 4,
        width: area.width / 2,
        height: area.height / 2,
    };

    f.render_widget(Clear, popup_area);
    f.render_widget(help_paragraph, popup_area);
}

/// Render a search popup for filtering stanzas.
pub fn render_search_popup(f: &mut Frame, search_query: &str, theme: &Theme) {
    let area = f.area();
    let popup_area = Rect {
        x: area.x + area.width / 4,
        y: area.y + area.height / 2 - 2,
        width: area.width / 2,
        height: 5,
    };

    let search_text = Paragraph::new(search_query).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Search Stanzas")
            .border_style(Style::default().fg(theme.border)),
    );

    f.render_widget(Clear, popup_area);
    f.render_widget(search_text, popup_area);
}
