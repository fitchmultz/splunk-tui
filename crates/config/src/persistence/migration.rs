//! Configuration migration logic.
//!
//! Responsibilities:
//! - Handle automatic migration from legacy to standard config paths.
//! - Atomic file operations using `std::fs::rename`.
//! - Best-effort migration that never blocks startup.
//!
//! Does NOT handle:
//! - Path determination (uses path module).
//! - Config file parsing.
//! - Profile operations.
//!
//! Invariants:
//! - Migration is atomic (uses rename).
//! - Never panics, never returns errors.
//! - Logs warnings on success/failure.

use std::path::Path;

/// If the target config does not exist but the legacy config exists, move it atomically.
///
/// - Uses `std::fs::rename` (atomic on same filesystem).
/// - Logs warnings on success/failure.
/// - Never panics and never returns errors: migration must not break startup.
pub(crate) fn migrate_config_file_if_needed(legacy_path: &Path, new_path: &Path) -> bool {
    if new_path.exists() {
        return false;
    }

    let legacy_meta = match std::fs::metadata(legacy_path) {
        Ok(m) => m,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return false;
            }
            tracing::warn!(
                legacy_path = %legacy_path.display(),
                error = %e,
                "Could not stat legacy config file; skipping migration"
            );
            return false;
        }
    };

    if !legacy_meta.is_file() {
        tracing::warn!(
            legacy_path = %legacy_path.display(),
            "Legacy config path exists but is not a file; skipping migration"
        );
        return false;
    }

    if let Some(parent) = new_path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(
            new_parent = %parent.display(),
            error = %e,
            "Could not create config directory for migrated config"
        );
        return false;
    }

    match std::fs::rename(legacy_path, new_path) {
        Ok(()) => {
            tracing::warn!(
                legacy_path = %legacy_path.display(),
                new_path = %new_path.display(),
                "Migrated config file from legacy path to new path"
            );
            true
        }
        Err(e) => {
            tracing::warn!(
                legacy_path = %legacy_path.display(),
                new_path = %new_path.display(),
                error = %e,
                "Could not migrate legacy config file"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    };
    use tempfile::TempDir;

    /// Minimal in-test tracing subscriber to capture WARN messages without adding dependencies.
    #[derive(Clone, Default)]
    struct CapturingSubscriber {
        events: Arc<Mutex<Vec<String>>>,
        next_id: Arc<AtomicU64>,
    }

    impl CapturingSubscriber {
        fn take_messages(&self) -> Vec<String> {
            std::mem::take(&mut *self.events.lock().expect("lock poisoned"))
        }
    }

    struct MessageVisitor {
        message: Option<String>,
    }

    impl MessageVisitor {
        fn new() -> Self {
            Self { message: None }
        }
    }

    impl tracing::field::Visit for MessageVisitor {
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            if field.name() == "message" {
                self.message = Some(value.to_string());
            }
        }

        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = Some(format!("{value:?}"));
            }
        }
    }

    impl tracing::Subscriber for CapturingSubscriber {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            tracing::span::Id::from_u64(id)
        }

        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

        fn event(&self, event: &tracing::Event<'_>) {
            let mut visitor = MessageVisitor::new();
            event.record(&mut visitor);
            if let Some(msg) = visitor.message {
                self.events.lock().expect("lock poisoned").push(msg);
            }
        }

        fn enter(&self, _span: &tracing::span::Id) {}

        fn exit(&self, _span: &tracing::span::Id) {}

        fn register_callsite(
            &self,
            _metadata: &'static tracing::Metadata<'static>,
        ) -> tracing::subscriber::Interest {
            tracing::subscriber::Interest::always()
        }

        fn clone_span(&self, id: &tracing::span::Id) -> tracing::span::Id {
            tracing::span::Id::from_u64(id.into_u64())
        }

        fn try_close(&self, _id: tracing::span::Id) -> bool {
            true
        }
    }

    fn capture_warn_messages<F: FnOnce()>(f: F) -> Vec<String> {
        let _guard = crate::test_util::global_test_lock().lock().unwrap();

        let subscriber = CapturingSubscriber {
            events: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        };

        let dispatch = tracing::Dispatch::new(subscriber.clone());
        tracing::dispatcher::with_default(&dispatch, f);
        subscriber.take_messages()
    }

    #[test]
    #[serial]
    fn test_migration_legacy_to_new_moves_file_and_preserves_content() {
        let temp_dir = TempDir::new().unwrap();

        let legacy_path = temp_dir
            .path()
            .join("splunk-tui")
            .join("splunk-tui")
            .join("config.json");
        let new_path = temp_dir.path().join("splunk-tui").join("config.json");

        std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        let content = r#"{ "profiles": { "default": { "base_url": "https://example:8089" } } }"#;
        std::fs::write(&legacy_path, content).unwrap();

        let messages = capture_warn_messages(|| {
            let migrated = migrate_config_file_if_needed(&legacy_path, &new_path);
            assert!(migrated, "expected migration to occur");
        });

        assert!(new_path.exists(), "new path should exist after migration");
        assert!(
            !legacy_path.exists(),
            "legacy path should be removed after migration"
        );
        assert_eq!(std::fs::read_to_string(&new_path).unwrap(), content);

        assert!(
            messages
                .iter()
                .any(|m| m.contains("Migrated config file from legacy path")),
            "expected migration to emit a warning log; got: {messages:?}"
        );
    }

    #[test]
    #[serial]
    fn test_migration_idempotent_second_run_noop() {
        let temp_dir = TempDir::new().unwrap();

        let legacy_path = temp_dir
            .path()
            .join("splunk-tui")
            .join("splunk-tui")
            .join("config.json");
        let new_path = temp_dir.path().join("splunk-tui").join("config.json");

        std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        std::fs::write(&legacy_path, r#"{"state":{"auto_refresh":true}}"#).unwrap();

        assert!(migrate_config_file_if_needed(&legacy_path, &new_path));
        assert!(!migrate_config_file_if_needed(&legacy_path, &new_path));

        assert!(new_path.exists());
        assert!(!legacy_path.exists());
    }

    #[test]
    #[serial]
    fn test_migration_failure_logged_but_not_fatal() {
        let temp_dir = TempDir::new().unwrap();

        // Legacy file exists.
        let legacy_path = temp_dir.path().join("legacy").join("config.json");
        std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        std::fs::write(&legacy_path, r#"{"state":{"auto_refresh":true}}"#).unwrap();

        // Make target parent a *file* so create_dir_all(parent) fails.
        let new_parent = temp_dir.path().join("newparent");
        std::fs::write(&new_parent, "i am a file, not a directory").unwrap();
        let new_path = new_parent.join("config.json");

        let messages = capture_warn_messages(|| {
            let migrated = migrate_config_file_if_needed(&legacy_path, &new_path);
            assert!(
                !migrated,
                "expected migration to fail gracefully (return false)"
            );
        });

        // Failure must not delete the legacy file.
        assert!(legacy_path.exists(), "legacy file should remain on failure");
        assert!(!new_path.exists(), "new file should not exist on failure");

        assert!(
            messages
                .iter()
                .any(|m| m.contains("Could not create config directory for migrated config")),
            "expected a warning log on migration failure; got: {messages:?}"
        );
    }
}
