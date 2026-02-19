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
