//! SHC screen rendering.
//!
//! Renders the SHC information including status, members, and captain info.
//! Supports toggling between summary and members views.
//!
//! Responsibilities:
//! - Render SHC summary information (status, captain URI, member count)
//! - Render SHC members as a table with status indicators
//! - Handle view mode switching (Summary vs Members)
//!
//! Does NOT handle:
//! - Does NOT fetch data (handled by async tasks in side_effects)
//! - Does NOT handle user input (handled by input module)

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    widgets::{Block, Borders, Cell, List, ListItem, Row, Table, TableState},
};
use splunk_client::models::{ShcMember, ShcStatus};
use splunk_config::Theme;

use crate::app::state::ShcViewMode;
use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_empty_state_custom, render_loading_state};

/// Configuration for rendering the SHC screen.
pub struct ShcRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The SHC status to display
    pub shc_status: Option<&'a ShcStatus>,
    /// The SHC members to display
    pub shc_members: Option<&'a [ShcMember]>,
    /// Current view mode
    pub view_mode: ShcViewMode,
    /// Table state for members view
    pub members_state: &'a mut TableState,
    /// Theme for consistent styling
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the SHC screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_shc(f: &mut Frame, area: Rect, config: ShcRenderConfig) {
    let ShcRenderConfig {
        loading,
        shc_status,
        shc_members,
        view_mode,
        members_state,
        theme,
        spinner_frame,
    } = config;

    if loading && shc_status.is_none() {
        render_loading_state(
            f,
            area,
            "SHC Information",
            "Loading SHC info...",
            spinner_frame,
            theme,
        );
        return;
    }

    let status = match shc_status {
        Some(s) => s,
        None => {
            render_empty_state(f, area, "SHC Information", "SHC info");
            return;
        }
    };

    match view_mode {
        ShcViewMode::Summary => {
            render_summary(f, area, status, theme);
        }
        ShcViewMode::Members => {
            render_members(f, area, status, shc_members, members_state, loading, theme);
        }
    }
}

/// Render the SHC summary view.
fn render_summary(f: &mut Frame, area: Rect, status: &ShcStatus, theme: &Theme) {
    let items: Vec<ListItem> = vec![
        ListItem::new(format!("Is Captain: {}", status.is_captain)),
        ListItem::new(format!("Is Searchable: {}", status.is_searchable)),
        ListItem::new(format!("Captain URI: {:?}", status.captain_uri)),
        ListItem::new(format!("Member Count: {}", status.member_count)),
        ListItem::new(format!(
            "Minimum Member Count: {:?}",
            status.minimum_member_count
        )),
        ListItem::new(format!(
            "Rolling Restart: {:?}",
            status.rolling_restart_flag
        )),
        ListItem::new(format!("Service Ready: {:?}", status.service_ready_flag)),
    ];

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("SHC Information (Summary) - Press 'm' for members")
            .border_style(theme.border())
            .title_style(theme.title()),
    );
    f.render_widget(list, area);
}

/// Render the SHC members view.
fn render_members(
    f: &mut Frame,
    area: Rect,
    _status: &ShcStatus,
    members: Option<&[ShcMember]>,
    state: &mut TableState,
    loading: bool,
    theme: &Theme,
) {
    let title = if loading {
        "SHC Members (Loading...)"
    } else {
        "SHC Members - Press 'm' for summary"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.border())
        .title_style(theme.title());

    let members = match members {
        Some(m) => m,
        None => {
            let message = if loading {
                "Loading members..."
            } else {
                "No members loaded. Press 'r' to refresh."
            };
            render_empty_state_custom(f, area, title, message);
            return;
        }
    };

    if members.is_empty() {
        let paragraph = ratatui::widgets::Paragraph::new("No SHC members found.")
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
        return;
    }

    // Define table headers
    let headers = ["Host", "Status", "Captain", "Port", "Site", "GUID"];

    // Create header row with styling
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::from(*h).style(theme.table_header()))
        .collect();
    let header = Row::new(header_cells).height(1);

    // Create rows for each member
    let rows: Vec<Row> = members
        .iter()
        .map(|member| {
            let host_text = if member.is_captain {
                format!("{} [C]", member.host)
            } else {
                member.host.clone()
            };

            let status_style = theme.status_style(&member.status);

            let cells = vec![
                Cell::from(host_text),
                Cell::from(member.status.clone()).style(status_style),
                Cell::from(if member.is_captain { "Yes" } else { "" }),
                Cell::from(member.port.to_string()),
                Cell::from(member.site.clone().unwrap_or_default()),
                Cell::from(member.guid.chars().take(8).collect::<String>()),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    // Column constraints
    let constraints = [
        Constraint::Min(20),    // Host (with captain indicator)
        Constraint::Length(12), // Status
        Constraint::Length(8),  // Captain
        Constraint::Length(6),  // Port
        Constraint::Length(10), // Site
        Constraint::Length(10), // GUID (first 8 chars)
    ];

    let table = Table::new(rows, constraints)
        .header(header)
        .block(block)
        .row_highlight_style(theme.highlight());

    f.render_stateful_widget(table, area, state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_member_status_style_up() {
        let theme = Theme::default();
        let style = theme.status_style("Up");
        // Should return success color
        assert_eq!(style.fg, Some(theme.success));
    }

    #[test]
    fn test_member_status_style_down() {
        let theme = Theme::default();
        let style = theme.status_style("Down");
        // Should return error color
        assert_eq!(style.fg, Some(theme.error));
    }

    #[test]
    fn test_member_status_style_pending() {
        let theme = Theme::default();
        let style = theme.status_style("Pending");
        // Should return warning color
        assert_eq!(style.fg, Some(theme.warning));
    }

    #[test]
    fn test_member_status_style_unknown() {
        let theme = Theme::default();
        let style = theme.status_style("Unknown");
        // Should return default text color
        assert_eq!(style.fg, Some(theme.text));
    }
}
