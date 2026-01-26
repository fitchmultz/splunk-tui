//! Cluster screen rendering.
//!
//! Renders the cluster information including ID, mode, and replication factors.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    widgets::{Block, Borders, List, ListItem},
};
use splunk_client::models::ClusterInfo;
use splunk_config::Theme;

/// Configuration for rendering the cluster screen.
pub struct ClusterRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The cluster information to display
    pub cluster_info: Option<&'a ClusterInfo>,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
}

/// Render the cluster screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_cluster(f: &mut Frame, area: Rect, config: ClusterRenderConfig) {
    let ClusterRenderConfig {
        loading,
        cluster_info,
        theme,
    } = config;

    if loading && cluster_info.is_none() {
        let loading_widget = ratatui::widgets::Paragraph::new("Loading cluster info...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Cluster Information"),
            )
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let info = match cluster_info {
        Some(i) => i,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No cluster info loaded. Press 'r' to refresh.")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Cluster Information"),
                    )
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    let items: Vec<ListItem> = vec![
        ListItem::new(format!("ID: {}", info.id)),
        ListItem::new(format!("Mode: {}", info.mode)),
        ListItem::new(format!("Label: {:?}", info.label)),
        ListItem::new(format!("Manager URI: {:?}", info.manager_uri)),
        ListItem::new(format!("Replication Factor: {:?}", info.replication_factor)),
        ListItem::new(format!("Search Factor: {:?}", info.search_factor)),
        ListItem::new(format!("Status: {:?}", info.status)),
    ];

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Cluster Information")
            .border_style(Style::default().fg(theme.border))
            .title_style(Style::default().fg(theme.title)),
    );
    f.render_widget(list, area);
}
