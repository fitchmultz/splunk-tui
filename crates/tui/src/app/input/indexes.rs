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
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch index data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::input::helpers::{
    handle_copy_with_toast, handle_list_export, is_copy_key, is_export_key, should_export_list,
};
use crate::ui::Toast;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle input for the indexes screen.
    pub fn handle_indexes_input(&mut self, key: KeyEvent) -> Option<Action> {
        if is_copy_key(key) {
            let content = self
                .indexes
                .as_ref()
                .and_then(|indexes| self.indexes_state.selected().and_then(|i| indexes.get(i)))
                .map(|idx| idx.name.clone());

            return handle_copy_with_toast(self, content);
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
            KeyCode::Char('e') if is_export_key(key) => {
                let can_export = should_export_list(self.indexes.as_ref());
                handle_list_export(self, can_export, ExportTarget::Indexes)
            }
            KeyCode::Char('c') => {
                // Open create index dialog
                Some(Action::OpenCreateIndexDialog)
            }
            KeyCode::Char('m') => {
                // Open modify index dialog for selected index
                if let Some(indexes) = &self.indexes
                    && let Some(selected) = self.indexes_state.selected()
                    && let Some(index) = indexes.get(selected)
                {
                    return Some(Action::OpenModifyIndexDialog {
                        name: index.name.clone(),
                    });
                }
                self.toasts.push(Toast::info("No index selected"));
                None
            }
            KeyCode::Char('d') => {
                // Open delete index confirmation for selected index
                if let Some(indexes) = &self.indexes
                    && let Some(selected) = self.indexes_state.selected()
                    && let Some(index) = indexes.get(selected)
                {
                    return Some(Action::OpenDeleteIndexConfirm {
                        name: index.name.clone(),
                    });
                }
                self.toasts.push(Toast::info("No index selected"));
                None
            }
            _ => None,
        }
    }
}
