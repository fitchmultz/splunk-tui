//! Cluster screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C copy of cluster ID
//! - Handle Ctrl+E export of cluster info
//! - Handle cluster management actions (maintenance mode, rebalance, decommission, remove)
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch cluster data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the cluster screen.
    pub fn handle_cluster_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy cluster ID
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            if let Some(info) = &self.cluster_info {
                return Some(Action::CopyToClipboard(info.id.clone()));
            }
            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL) && self.cluster_info.is_some() =>
            {
                self.begin_export(ExportTarget::ClusterInfo);
                None
            }
            // Maintenance mode toggle (m)
            KeyCode::Char('m') => {
                if let Some(info) = &self.cluster_info {
                    let enable = info.maintenance_mode != Some(true);
                    return Some(Action::SetMaintenanceMode { enable });
                }
                self.toasts.push(Toast::info("No cluster info available"));
                None
            }
            // Rebalance cluster (r)
            KeyCode::Char('r') => Some(Action::RebalanceCluster),
            // Decommission peer (d) - only in Peers view
            KeyCode::Char('d') => {
                if let Some(peer) = self.get_selected_cluster_peer() {
                    return Some(Action::DecommissionPeer {
                        peer_guid: peer.guid.clone(),
                    });
                }
                self.toasts.push(Toast::info("No peer selected"));
                None
            }
            // Remove peer (x) - only in Peers view
            KeyCode::Char('x') => {
                if let Some(peer) = self.get_selected_cluster_peer() {
                    return Some(Action::RemovePeer {
                        peer_guid: peer.guid.clone(),
                    });
                }
                self.toasts.push(Toast::info("No peer selected"));
                None
            }
            _ => None,
        }
    }
}
