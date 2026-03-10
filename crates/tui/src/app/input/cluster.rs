//! Cluster screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C copy of cluster ID
//! - Handle Ctrl+E export of cluster info
//! - Handle cluster management actions (maintenance mode, rebalance, decommission, remove)
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch cluster data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::input::helpers::{
    handle_copy_with_toast, handle_single_export, is_copy_key, is_export_key, should_export_single,
};
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle input for the cluster screen.
    pub fn handle_cluster_input(&mut self, key: KeyEvent) -> Option<Action> {
        if is_copy_key(key) {
            let content = self.cluster_info.as_ref().map(|info| info.id.clone());
            return handle_copy_with_toast(self, content);
        }

        match key.code {
            KeyCode::Char('e') if is_export_key(key) => {
                let can_export = should_export_single(self.cluster_info.as_ref());
                handle_single_export(self, can_export, ExportTarget::ClusterInfo)
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
