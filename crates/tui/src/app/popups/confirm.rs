//! Confirmation dialog popup handlers.
//!
//! Responsibilities:
//! - Handle confirmation dialogs for job/app/user/index operations
//! - Confirm or cancel destructive actions
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT execute the actions (just returns Action variants)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle confirmation dialog popups.
    pub fn handle_confirm_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            // ConfirmCancel
            (Some(PopupType::ConfirmCancel(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sid = if let Some(Popup {
                    kind: PopupType::ConfirmCancel(s),
                    ..
                }) = self.popup.take()
                {
                    s
                } else {
                    unreachable!()
                };
                Some(Action::CancelJob(sid))
            }
            // ConfirmDelete
            (Some(PopupType::ConfirmDelete(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sid = if let Some(Popup {
                    kind: PopupType::ConfirmDelete(s),
                    ..
                }) = self.popup.take()
                {
                    s
                } else {
                    unreachable!()
                };
                Some(Action::DeleteJob(sid))
            }
            // ConfirmCancelBatch
            (Some(PopupType::ConfirmCancelBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::CancelJobsBatch(sids))
            }
            // ConfirmDeleteBatch
            (Some(PopupType::ConfirmDeleteBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::DeleteJobsBatch(sids))
            }
            // ConfirmEnableApp
            (Some(PopupType::ConfirmEnableApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmEnableApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::EnableApp(name))
            }
            // ConfirmDisableApp
            (Some(PopupType::ConfirmDisableApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmDisableApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::DisableApp(name))
            }
            // ConfirmRemoveApp
            (Some(PopupType::ConfirmRemoveApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmRemoveApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::RemoveApp { app_name: name })
            }
            // DeleteIndexConfirm
            (
                Some(PopupType::DeleteIndexConfirm { index_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = index_name.clone();
                self.popup = None;
                Some(Action::DeleteIndex { name })
            }
            // DeleteUserConfirm
            (
                Some(PopupType::DeleteUserConfirm { user_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = user_name.clone();
                self.popup = None;
                Some(Action::DeleteUser { name })
            }
            // DeleteLookupConfirm
            (
                Some(PopupType::DeleteLookupConfirm { lookup_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = lookup_name.clone();
                self.popup = None;
                Some(Action::DeleteLookup {
                    name,
                    app: None,
                    owner: None,
                })
            }
            // Reject confirmations with 'n' or Esc
            (
                Some(
                    PopupType::ConfirmCancel(_)
                    | PopupType::ConfirmDelete(_)
                    | PopupType::ConfirmCancelBatch(_)
                    | PopupType::ConfirmDeleteBatch(_)
                    | PopupType::ConfirmEnableApp(_)
                    | PopupType::ConfirmDisableApp(_)
                    | PopupType::ConfirmRemoveApp(_),
                ),
                KeyCode::Char('n') | KeyCode::Esc,
            ) => {
                self.popup = None;
                None
            }
            (Some(PopupType::DeleteIndexConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::DeleteUserConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::DeleteLookupConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }
}
