//! Application state and rendering.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// Current active screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    Search,
    Indexes,
    Cluster,
}

/// Application state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AppState {
    Running,
    Quitting,
}

/// Main application state.
pub struct App {
    pub state: AppState,
    pub current_screen: CurrentScreen,
    pub search_input: String,
    pub search_status: String,
    pub indexes: Vec<String>,
    pub cluster_info: Vec<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::Running,
            current_screen: CurrentScreen::Search,
            search_input: String::new(),
            search_status: String::from("Press Enter to execute search"),
            indexes: vec![
                "main - Primary index".to_string(),
                "_internal - Splunk internal logs".to_string(),
                "_audit - Audit logs".to_string(),
            ],
            cluster_info: vec![
                "Mode: Standalone".to_string(),
                "Status: Not clustered".to_string(),
            ],
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Header
                    Constraint::Min(0),    // Main content
                    Constraint::Length(3), // Footer
                ]
                .as_ref(),
            )
            .split(f.area());

        // Header
        let header = Paragraph::new(vec![Line::from(vec![
            Span::styled(
                "Splunk TUI",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled(
                match self.current_screen {
                    CurrentScreen::Search => "Search",
                    CurrentScreen::Indexes => "Indexes",
                    CurrentScreen::Cluster => "Cluster",
                },
                Style::default().fg(Color::Yellow),
            ),
        ])])
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Main content
        self.render_content(f, chunks[1]);

        // Footer
        let footer_text = vec![Line::from(vec![
            Span::raw(" 1:Search 2:Indexes 3:Cluster "),
            Span::raw("|"),
            Span::styled(" q:Quit ", Style::default().fg(Color::Red)),
        ])];
        let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        match self.current_screen {
            CurrentScreen::Search => self.render_search(f, area),
            CurrentScreen::Indexes => self.render_indexes(f, area),
            CurrentScreen::Cluster => self.render_cluster(f, area),
        }
    }

    fn render_search(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Search input
                    Constraint::Length(3), // Status
                    Constraint::Min(0),    // Results
                ]
                .as_ref(),
            )
            .split(area);

        // Search input
        let input = Paragraph::new(self.search_input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Search Query"));
        f.render_widget(input, chunks[0]);

        // Status
        let status = Paragraph::new(self.search_status.as_str())
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, chunks[1]);

        // Results placeholder
        let results = Paragraph::new("Search results will appear here")
            .block(Block::default().borders(Borders::ALL).title("Results"));
        f.render_widget(results, chunks[2]);
    }

    fn render_indexes(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .indexes
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Indexes"));
        f.render_widget(list, area);
    }

    fn render_cluster(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .cluster_info
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Cluster Information"),
        );
        f.render_widget(list, area);
    }
}
