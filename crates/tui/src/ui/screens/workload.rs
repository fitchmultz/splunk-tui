//! Workload management screen for the TUI.
//!
//! Responsibilities:
//! - Render workload pools table with resource allocation info
//! - Render workload rules table with matching criteria
//! - Display loading state and empty state
//! - Support toggling between Pools and Rules views
//!
//! Does NOT handle:
//! - Data fetching (handled by side effects)
//! - User input (handled by input handlers)

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};
use splunk_client::models::{WorkloadPool, WorkloadRule};
use splunk_config::Theme;

use crate::app::state::WorkloadViewMode;
use crate::ui::theme::{ThemeExt, spinner_char};

/// Configuration for rendering the workload management screen.
pub struct WorkloadRenderConfig<'a> {
    /// Whether data is currently loading.
    pub loading: bool,
    /// The list of workload pools to display.
    pub workload_pools: Option<&'a [WorkloadPool]>,
    /// The list of workload rules to display.
    pub workload_rules: Option<&'a [WorkloadRule]>,
    /// Current view mode.
    pub view_mode: WorkloadViewMode,
    /// Table state for pools view.
    pub pools_state: &'a mut TableState,
    /// Table state for rules view.
    pub rules_state: &'a mut TableState,
    /// The theme to use for styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the workload management screen.
pub fn render_workload(f: &mut Frame, area: Rect, config: WorkloadRenderConfig) {
    let WorkloadRenderConfig {
        loading,
        workload_pools,
        workload_rules,
        view_mode,
        pools_state,
        rules_state,
        theme,
        spinner_frame,
    } = config;

    if loading && workload_pools.is_none() && workload_rules.is_none() {
        let spinner = spinner_char(spinner_frame);
        let loading_widget = Paragraph::new(format!("{} Loading workload management...", spinner))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Workload Management"),
            )
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    match view_mode {
        WorkloadViewMode::Pools => {
            render_pools(f, area, workload_pools, pools_state, loading, theme);
        }
        WorkloadViewMode::Rules => {
            render_rules(f, area, workload_rules, rules_state, loading, theme);
        }
    }
}

/// Render the workload pools view.
fn render_pools(
    f: &mut Frame,
    area: Rect,
    pools: Option<&[WorkloadPool]>,
    state: &mut TableState,
    loading: bool,
    theme: &Theme,
) {
    let title = if loading {
        "Workload Pools (Loading...) - Press 'w' for rules"
    } else {
        "Workload Pools - Press 'w' for rules"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.border())
        .title_style(theme.title());

    let pools = match pools {
        Some(p) => p,
        None => {
            let paragraph = Paragraph::new(if loading {
                "Loading pools..."
            } else {
                "No pools loaded. Press 'r' to refresh."
            })
            .block(block)
            .alignment(Alignment::Center);
            f.render_widget(paragraph, area);
            return;
        }
    };

    if pools.is_empty() {
        let paragraph = Paragraph::new("No workload pools found.")
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
        return;
    }

    // Define table headers
    let headers = [
        "Name",
        "CPU Weight",
        "Mem Weight",
        "Default",
        "Enabled",
        "Concurrency",
    ];

    // Create header row with styling
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::from(*h).style(theme.table_header()))
        .collect();
    let header = Row::new(header_cells).height(1);

    // Create rows for each pool
    let rows: Vec<Row> = pools
        .iter()
        .map(|pool| {
            let cells = vec![
                Cell::from(pool.name.clone()),
                Cell::from(pool.cpu_weight.map(|w| w.to_string()).unwrap_or_default()),
                Cell::from(pool.mem_weight.map(|w| w.to_string()).unwrap_or_default()),
                Cell::from(
                    pool.default_pool
                        .map(|d| if d { "Yes" } else { "" })
                        .unwrap_or(""),
                ),
                Cell::from(
                    pool.enabled
                        .map(|e| if e { "Yes" } else { "No" })
                        .unwrap_or(""),
                ),
                Cell::from(
                    pool.search_concurrency
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                ),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    // Column constraints
    let constraints = [
        Constraint::Min(20),    // Name
        Constraint::Length(12), // CPU Weight
        Constraint::Length(12), // Mem Weight
        Constraint::Length(8),  // Default
        Constraint::Length(8),  // Enabled
        Constraint::Length(12), // Concurrency
    ];

    let table = Table::new(rows, constraints)
        .header(header)
        .block(block)
        .row_highlight_style(theme.highlight());

    f.render_stateful_widget(table, area, state);
}

/// Render the workload rules view.
fn render_rules(
    f: &mut Frame,
    area: Rect,
    rules: Option<&[WorkloadRule]>,
    state: &mut TableState,
    loading: bool,
    theme: &Theme,
) {
    let title = if loading {
        "Workload Rules (Loading...) - Press 'w' for pools"
    } else {
        "Workload Rules - Press 'w' for pools"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.border())
        .title_style(theme.title());

    let rules = match rules {
        Some(r) => r,
        None => {
            let paragraph = Paragraph::new(if loading {
                "Loading rules..."
            } else {
                "No rules loaded. Press 'r' to refresh."
            })
            .block(block)
            .alignment(Alignment::Center);
            f.render_widget(paragraph, area);
            return;
        }
    };

    if rules.is_empty() {
        let paragraph = Paragraph::new("No workload rules found.")
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
        return;
    }

    // Define table headers
    let headers = ["Name", "Pool", "User", "App", "Type", "Enabled", "Order"];

    // Create header row with styling
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::from(*h).style(theme.table_header()))
        .collect();
    let header = Row::new(header_cells).height(1);

    // Create rows for each rule
    let rows: Vec<Row> = rules
        .iter()
        .map(|rule| {
            let cells = vec![
                Cell::from(rule.name.clone()),
                Cell::from(rule.workload_pool.clone().unwrap_or_default()),
                Cell::from(rule.user.clone().unwrap_or_default()),
                Cell::from(rule.app.clone().unwrap_or_default()),
                Cell::from(rule.search_type.clone().unwrap_or_default()),
                Cell::from(
                    rule.enabled
                        .map(|e| if e { "Yes" } else { "No" })
                        .unwrap_or(""),
                ),
                Cell::from(rule.order.map(|o| o.to_string()).unwrap_or_default()),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    // Column constraints
    let constraints = [
        Constraint::Min(20),    // Name
        Constraint::Min(15),    // Pool
        Constraint::Length(15), // User
        Constraint::Length(15), // App
        Constraint::Length(12), // Type
        Constraint::Length(8),  // Enabled
        Constraint::Length(8),  // Order
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
    fn test_workload_view_mode_default() {
        let mode: WorkloadViewMode = Default::default();
        assert_eq!(mode, WorkloadViewMode::Pools);
    }

    #[test]
    fn test_workload_view_mode_toggle() {
        // Start with Pools, toggle to Rules
        let mode = WorkloadViewMode::Pools;
        let toggled = mode.toggle();
        assert_eq!(toggled, WorkloadViewMode::Rules);

        // Toggle back to Pools
        let toggled_back = toggled.toggle();
        assert_eq!(toggled_back, WorkloadViewMode::Pools);
    }

    #[test]
    fn test_workload_view_mode_toggle_cycle() {
        // Verify that toggling twice returns to original state
        let mode = WorkloadViewMode::Pools;
        let after_two_toggles = mode.toggle().toggle();
        assert_eq!(mode, after_two_toggles);
    }
}
