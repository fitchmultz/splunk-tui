//! Index management popup handlers.
//!
//! Responsibilities:
//! - Handle index creation, modification, and deletion popups
//! - Handle index details view with scrolling and copy-to-clipboard
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT execute the actual index operations (just returns Action variants)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle index-related popups (CreateIndex, ModifyIndex, DeleteIndexConfirm, IndexDetails).
    pub fn handle_index_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            // IndexDetails - close
            (Some(PopupType::IndexDetails), KeyCode::Esc | KeyCode::Char('q')) => {
                self.popup = None;
                self.index_details_scroll_offset = 0;
                None
            }
            // IndexDetails - scroll navigation
            (Some(PopupType::IndexDetails), KeyCode::Char('j') | KeyCode::Down) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_add(1);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::Char('k') | KeyCode::Up) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_sub(1);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::PageDown) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_add(10);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::PageUp) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_sub(10);
                None
            }
            // IndexDetails - copy to clipboard
            (Some(PopupType::IndexDetails), KeyCode::Char('c'))
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if let Some(indexes) = &self.indexes
                    && let Some(selected) = self.indexes_state.selected()
                    && let Some(index) = indexes.get(selected)
                    && let Ok(json) = serde_json::to_string_pretty(index)
                {
                    return Some(Action::CopyToClipboard(json));
                }
                None
            }
            // CreateIndex - close
            (Some(PopupType::CreateIndex { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // CreateIndex - submit
            (Some(PopupType::CreateIndex { name_input, .. }), KeyCode::Enter) => {
                if name_input.is_empty() {
                    return None;
                }
                let name = name_input.clone();
                // Extract other fields from the popup state
                if let Some(Popup {
                    kind:
                        PopupType::CreateIndex {
                            max_data_size_mb,
                            max_hot_buckets,
                            max_warm_db_count,
                            frozen_time_period_secs,
                            home_path,
                            cold_db_path,
                            thawed_path,
                            cold_to_frozen_dir,
                            ..
                        },
                    ..
                }) = self.popup.take()
                {
                    let params = splunk_client::CreateIndexParams {
                        name,
                        max_data_size_mb,
                        max_hot_buckets,
                        max_warm_db_count,
                        frozen_time_period_in_secs: frozen_time_period_secs,
                        home_path,
                        cold_db_path,
                        thawed_path,
                        cold_to_frozen_dir,
                    };
                    Some(Action::CreateIndex { params })
                } else {
                    None
                }
            }
            // CreateIndex - character input
            (
                Some(PopupType::CreateIndex {
                    name_input,
                    max_data_size_mb,
                    max_hot_buckets,
                    max_warm_db_count,
                    frozen_time_period_secs,
                    home_path,
                    cold_db_path,
                    thawed_path,
                    cold_to_frozen_dir,
                }),
                KeyCode::Char(c),
            ) => {
                let mut new_name = name_input.clone();
                new_name.push(c);
                self.popup = Some(
                    Popup::builder(PopupType::CreateIndex {
                        name_input: new_name,
                        max_data_size_mb: *max_data_size_mb,
                        max_hot_buckets: *max_hot_buckets,
                        max_warm_db_count: *max_warm_db_count,
                        frozen_time_period_secs: *frozen_time_period_secs,
                        home_path: home_path.clone(),
                        cold_db_path: cold_db_path.clone(),
                        thawed_path: thawed_path.clone(),
                        cold_to_frozen_dir: cold_to_frozen_dir.clone(),
                    })
                    .build(),
                );
                None
            }
            // CreateIndex - backspace
            (
                Some(PopupType::CreateIndex {
                    name_input,
                    max_data_size_mb,
                    max_hot_buckets,
                    max_warm_db_count,
                    frozen_time_period_secs,
                    home_path,
                    cold_db_path,
                    thawed_path,
                    cold_to_frozen_dir,
                }),
                KeyCode::Backspace,
            ) => {
                let mut new_name = name_input.clone();
                new_name.pop();
                self.popup = Some(
                    Popup::builder(PopupType::CreateIndex {
                        name_input: new_name,
                        max_data_size_mb: *max_data_size_mb,
                        max_hot_buckets: *max_hot_buckets,
                        max_warm_db_count: *max_warm_db_count,
                        frozen_time_period_secs: *frozen_time_period_secs,
                        home_path: home_path.clone(),
                        cold_db_path: cold_db_path.clone(),
                        thawed_path: thawed_path.clone(),
                        cold_to_frozen_dir: cold_to_frozen_dir.clone(),
                    })
                    .build(),
                );
                None
            }
            // ModifyIndex - close
            (Some(PopupType::ModifyIndex { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // ModifyIndex - submit
            (Some(PopupType::ModifyIndex { index_name, .. }), KeyCode::Enter) => {
                let name = index_name.clone();
                if let Some(Popup {
                    kind:
                        PopupType::ModifyIndex {
                            new_max_data_size_mb,
                            new_max_hot_buckets,
                            new_max_warm_db_count,
                            new_frozen_time_period_secs,
                            new_home_path,
                            new_cold_db_path,
                            new_thawed_path,
                            new_cold_to_frozen_dir,
                            ..
                        },
                    ..
                }) = self.popup.take()
                {
                    let params = splunk_client::ModifyIndexParams {
                        max_data_size_mb: new_max_data_size_mb,
                        max_hot_buckets: new_max_hot_buckets,
                        max_warm_db_count: new_max_warm_db_count,
                        frozen_time_period_in_secs: new_frozen_time_period_secs,
                        home_path: new_home_path,
                        cold_db_path: new_cold_db_path,
                        thawed_path: new_thawed_path,
                        cold_to_frozen_dir: new_cold_to_frozen_dir,
                    };
                    Some(Action::ModifyIndex { name, params })
                } else {
                    None
                }
            }
            // DeleteIndexConfirm - cancel
            (Some(PopupType::DeleteIndexConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }
}
