//! Undo history popup input handler.
//!
//! Responsibilities:
//! - Handle scrolling in undo history popup
//! - Handle close actions (Esc, q)
//!
//! Does NOT handle:
//! - Does NOT render the popup (handled by ui::popup module)

use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle keyboard input for the undo history popup.
    pub fn handle_undo_history_popup(&mut self, key: KeyEvent) -> Option<crate::action::Action> {
        match key.code {
            // Close popup
            KeyCode::Esc | KeyCode::Char('q') => {
                self.popup = None;
                None
            }

            // Scroll down
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(crate::ui::popup::Popup {
                    kind: crate::ui::popup::PopupType::UndoHistory { scroll_offset },
                    ..
                }) = self.popup.as_mut()
                {
                    *scroll_offset = scroll_offset.saturating_add(1);
                }
                None
            }

            // Scroll up
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(crate::ui::popup::Popup {
                    kind: crate::ui::popup::PopupType::UndoHistory { scroll_offset },
                    ..
                }) = self.popup.as_mut()
                {
                    *scroll_offset = scroll_offset.saturating_sub(1);
                }
                None
            }

            // Page down
            KeyCode::PageDown => {
                if let Some(crate::ui::popup::Popup {
                    kind: crate::ui::popup::PopupType::UndoHistory { scroll_offset },
                    ..
                }) = self.popup.as_mut()
                {
                    *scroll_offset = scroll_offset.saturating_add(10);
                }
                None
            }

            // Page up
            KeyCode::PageUp => {
                if let Some(crate::ui::popup::Popup {
                    kind: crate::ui::popup::PopupType::UndoHistory { scroll_offset },
                    ..
                }) = self.popup.as_mut()
                {
                    *scroll_offset = scroll_offset.saturating_sub(10);
                }
                None
            }

            // Go to top
            KeyCode::Home => {
                if let Some(crate::ui::popup::Popup {
                    kind: crate::ui::popup::PopupType::UndoHistory { scroll_offset },
                    ..
                }) = self.popup.as_mut()
                {
                    *scroll_offset = 0;
                }
                None
            }

            // Go to bottom - just set to a large number, will be clamped by rendering
            KeyCode::End => {
                if let Some(crate::ui::popup::Popup {
                    kind: crate::ui::popup::PopupType::UndoHistory { scroll_offset },
                    ..
                }) = self.popup.as_mut()
                {
                    *scroll_offset = usize::MAX;
                }
                None
            }

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::app::App;
    use crate::app::ConnectionContext;
    use crate::ui::popup::{Popup, PopupType};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_test_app() -> App {
        App::new(None, ConnectionContext::default())
    }

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::empty())
    }

    #[test]
    fn test_undo_history_close_with_esc() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::UndoHistory { scroll_offset: 0 }).build());

        let action = app.handle_undo_history_popup(key(KeyCode::Esc));

        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_undo_history_close_with_q() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::UndoHistory { scroll_offset: 0 }).build());

        let action = app.handle_undo_history_popup(key(KeyCode::Char('q')));

        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_undo_history_scroll_down() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::UndoHistory { scroll_offset: 0 }).build());

        app.handle_undo_history_popup(key(KeyCode::Char('j')));

        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::UndoHistory { scroll_offset: 1 },
                ..
            })
        ));
    }

    #[test]
    fn test_undo_history_scroll_up() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::UndoHistory { scroll_offset: 5 }).build());

        app.handle_undo_history_popup(key(KeyCode::Char('k')));

        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::UndoHistory { scroll_offset: 4 },
                ..
            })
        ));
    }

    #[test]
    fn test_undo_history_scroll_up_at_zero() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::UndoHistory { scroll_offset: 0 }).build());

        app.handle_undo_history_popup(key(KeyCode::Char('k')));

        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::UndoHistory { scroll_offset: 0 },
                ..
            })
        ));
    }

    #[test]
    fn test_undo_history_page_down() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::UndoHistory { scroll_offset: 0 }).build());

        app.handle_undo_history_popup(key(KeyCode::PageDown));

        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::UndoHistory { scroll_offset: 10 },
                ..
            })
        ));
    }

    #[test]
    fn test_undo_history_page_up() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::UndoHistory { scroll_offset: 15 }).build());

        app.handle_undo_history_popup(key(KeyCode::PageUp));

        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::UndoHistory { scroll_offset: 5 },
                ..
            })
        ));
    }

    #[test]
    fn test_undo_history_go_to_top() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::UndoHistory { scroll_offset: 50 }).build());

        app.handle_undo_history_popup(key(KeyCode::Home));

        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::UndoHistory { scroll_offset: 0 },
                ..
            })
        ));
    }
}
