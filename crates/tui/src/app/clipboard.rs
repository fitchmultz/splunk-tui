//! Clipboard integration for the TUI App module.
//!
//! Responsibilities:
//! - Provide `copy_to_clipboard(String) -> Result<(), String>` backed by `arboard`.
//! - Provide a test-only (but always compiled) per-thread override backend so
//!   integration tests do not depend on the host OS clipboard.
//!
//! Does NOT handle:
//! - Does NOT implement per-screen "what should be copied" decisions (handled by `App`).
//! - Does NOT manage UI feedback (toasts) (handled by `App::update`).
//!
//! Invariants / assumptions:
//! - Called from the main UI threads (typical TUI event loop).
//! - If the OS clipboard is unavailable, this module returns an error instead of panicking.

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

/// Thread-local clipboard backend override (used by tests to avoid OS clipboard dependencies).
#[derive(Clone)]
enum OverrideBackend {
    Recording(Arc<Mutex<Option<String>>>),
    Failing(String),
}

thread_local! {
    static OVERRIDE_BACKEND: RefCell<Option<OverrideBackend>> = const { RefCell::new(None) };
}

/// Copy the given content to the system clipboard.
///
/// Returns an error string if clipboard initialization or writes fail.
///
/// Note: In tests, a per-thread override backend may be installed to make the
/// behavior deterministic.
pub fn copy_to_clipboard(content: String) -> Result<(), String> {
    OVERRIDE_BACKEND.with(|cell| {
        if let Some(backend) = cell.borrow().as_ref() {
            match backend {
                OverrideBackend::Recording(store) => {
                    let mut guard = store
                        .lock()
                        .map_err(|_| "Clipboard test backend lock poisoned".to_string())?;
                    *guard = Some(content);
                    return Ok(());
                }
                OverrideBackend::Failing(msg) => return Err(msg.clone()),
            }
        }

        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("Clipboard unavailable: {e}"))?;
        clipboard
            .set_text(content)
            .map_err(|e| format!("Failed to write to clipboard: {e}"))?;
        Ok(())
    })
}

/// Installs a per-thread recording clipboard backend and returns a guard.
///
/// This is primarily for integration tests so they don't require an OS clipboard.
///
/// The guard restores any previous backend on drop.
///
/// The returned guard can be queried for the last copied text.
#[doc(hidden)]
pub fn install_recording_clipboard() -> RecordingClipboardGuard {
    let store: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let prev = OVERRIDE_BACKEND.with(|cell| {
        cell.borrow_mut()
            .replace(OverrideBackend::Recording(store.clone()))
    });

    RecordingClipboardGuard { prev, store }
}

/// Installs a per-thread failing clipboard backend and returns a guard.
///
/// This is used to test error-path behavior deterministically.
///
/// The guard restores any previous backend on drop.
#[doc(hidden)]
pub fn install_failing_clipboard(message: impl Into<String>) -> FailingClipboardGuard {
    let prev = OVERRIDE_BACKEND.with(|cell| {
        cell.borrow_mut()
            .replace(OverrideBackend::Failing(message.into()))
    });

    FailingClipboardGuard { prev }
}

/// RAII guard for a recording clipboard backend.
#[doc(hidden)]
pub struct RecordingClipboardGuard {
    prev: Option<OverrideBackend>,
    store: Arc<Mutex<Option<String>>>,
}

impl RecordingClipboardGuard {
    /// Returns the last copied text recorded by this backend.
    pub fn copied_text(&self) -> Option<String> {
        self.store.lock().ok().and_then(|g| g.clone())
    }
}

impl Drop for RecordingClipboardGuard {
    fn drop(&mut self) {
        let prev = self.prev.take();
        OVERRIDE_BACKEND.with(|cell| {
            *cell.borrow_mut() = prev;
        });
    }
}

/// RAII guard for a failing clipboard backend.
#[doc(hidden)]
pub struct FailingClipboardGuard {
    prev: Option<OverrideBackend>,
}

impl Drop for FailingClipboardGuard {
    fn drop(&mut self) {
        let prev = self.prev.take();
        OVERRIDE_BACKEND.with(|cell| {
            *cell.borrow_mut() = prev;
        });
    }
}
