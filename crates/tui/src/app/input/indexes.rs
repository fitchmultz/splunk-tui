//! Indexes screen input handler.
//!
//! Responsibilities:
//! - Handle Enter key to show index details popup
//! - Handle Ctrl+C copy of selected index name
//! - Handle Ctrl+E export of indexes list
//! - Handle 'c' to open create index dialog
//! - Handle 'm' to open modify index dialog
//! - Handle 'd' to open delete index confirmation
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch index data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the indexes screen.
    pub fn handle_indexes_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C or 'y': copy selected index name (vim-style)
        let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
            || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')));
        if is_copy {
            let content = self
                .indexes
                .as_ref()
                .and_then(|indexes| self.indexes_state.selected().and_then(|i| indexes.get(i)))
                .map(|idx| idx.name.clone());

            if let Some(content) = content {
                return Some(Action::CopyToClipboard(content));
            }

            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Enter => {
                if let Some(indexes) = &self.indexes
                    && let Some(selected) = self.indexes_state.selected()
                    && indexes.get(selected).is_some()
                {
                    self.popup = Some(Popup::builder(PopupType::IndexDetails).build());
                    self.index_details_scroll_offset = 0;
                }
                None
            }
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self
                        .indexes
                        .as_ref()
                        .map(|v| !v.is_empty())
                        .unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::Indexes);
                None
            }
            KeyCode::Char('c') => {
                // Open create index dialog
                self.popup = Some(
                    Popup::builder(PopupType::CreateIndex {
                        name_input: String::new(),
                        max_data_size_mb: None,
                        max_hot_buckets: None,
                        max_warm_db_count: None,
                        frozen_time_period_secs: None,
                        home_path: None,
                        cold_db_path: None,
                        thawed_path: None,
                        cold_to_frozen_dir: None,
                    })
                    .build(),
                );
                None
            }
            KeyCode::Char('m') => {
                // Open modify index dialog for selected index
                if let Some(indexes) = &self.indexes
                    && let Some(selected) = self.indexes_state.selected()
                    && let Some(index) = indexes.get(selected)
                {
                    self.popup = Some(
                        Popup::builder(PopupType::ModifyIndex {
                            index_name: index.name.clone(),
                            current_max_data_size_mb: index.max_total_data_size_mb,
                            current_max_hot_buckets: index
                                .max_hot_buckets
                                .as_ref()
                                .and_then(|s| s.parse().ok()),
                            current_max_warm_db_count: index.max_warm_db_count,
                            current_frozen_time_period_secs: index.frozen_time_period_in_secs,
                            current_home_path: index.home_path.clone(),
                            current_cold_db_path: index.cold_db_path.clone(),
                            current_thawed_path: index.thawed_path.clone(),
                            current_cold_to_frozen_dir: index.cold_to_frozen_dir.clone(),
                            new_max_data_size_mb: index.max_total_data_size_mb,
                            new_max_hot_buckets: index
                                .max_hot_buckets
                                .as_ref()
                                .and_then(|s| s.parse().ok()),
                            new_max_warm_db_count: index.max_warm_db_count,
                            new_frozen_time_period_secs: index.frozen_time_period_in_secs,
                            new_home_path: index.home_path.clone(),
                            new_cold_db_path: index.cold_db_path.clone(),
                            new_thawed_path: index.thawed_path.clone(),
                            new_cold_to_frozen_dir: index.cold_to_frozen_dir.clone(),
                        })
                        .build(),
                    );
                } else {
                    self.toasts.push(Toast::info("No index selected"));
                }
                None
            }
            KeyCode::Char('d') => {
                // Open delete index confirmation for selected index
                if let Some(indexes) = &self.indexes
                    && let Some(selected) = self.indexes_state.selected()
                    && let Some(index) = indexes.get(selected)
                {
                    self.popup = Some(
                        Popup::builder(PopupType::DeleteIndexConfirm {
                            index_name: index.name.clone(),
                        })
                        .build(),
                    );
                } else {
                    self.toasts.push(Toast::info("No index selected"));
                }
                None
            }
            _ => None,
        }
    }
}
