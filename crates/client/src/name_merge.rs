//! Entry-name merging helpers for Splunk list endpoints.
//!
//! Responsibilities:
//! - Splunk list endpoints commonly return the resource name at `entry[].name`, not inside `entry[].content`.
//! - This module centralizes the logic to copy `entry.name` into the deserialized content model.
//!
//! Explicitly does NOT handle:
//! - Any other normalization (type coercion, field defaults beyond the `name` field).
//! - Validation of name formats.
//!
//! Invariants / assumptions:
//! - Callers should invoke this for endpoints where `content.name` may be missing/empty.
//! - This is crate-internal glue; it is not part of the public API contract.

use crate::models::{App, Forwarder, Index, Input, SavedSearch, SearchPeer, User};

pub(crate) trait HasName {
    fn set_name(&mut self, name: String);
}

pub(crate) fn attach_entry_name<T: HasName>(entry_name: String, mut content: T) -> T {
    content.set_name(entry_name);
    content
}

impl HasName for Index {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl HasName for App {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl HasName for User {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl HasName for SavedSearch {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl HasName for Forwarder {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl HasName for SearchPeer {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl HasName for Input {
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attach_entry_name_sets_name() {
        let index = Index {
            name: String::new(),
            max_total_data_size_mb: None,
            current_db_size_mb: 0,
            total_event_count: 0,
            max_warm_db_count: None,
            max_hot_buckets: None,
            frozen_time_period_in_secs: None,
            cold_db_path: None,
            home_path: None,
            thawed_path: None,
            cold_to_frozen_dir: None,
            primary_index: None,
        };

        let index = attach_entry_name("main".to_string(), index);
        assert_eq!(index.name, "main");
    }
}
