//! Shared popup helpers for state mutation and input normalization.
//!
//! Responsibilities:
//! - Rebuild popup state after in-place logical updates.
//! - Convert form strings into optional persisted values.
//! - Provide checked numeric editing helpers for popup forms.
//!
//! Does NOT handle:
//! - Popup-specific business rules.
//! - Popup rendering.
//! - Action dispatch outside popup state updates.
//!
//! Invariants:
//! - Popup rebuilds always go through `Popup::builder`.
//! - Numeric edits never overflow silently.

use secrecy::SecretString;
use splunk_config::types::SecureValue;

use crate::app::App;
use crate::ui::popup::{Popup, PopupType};

pub(super) fn optional_string(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

pub(super) fn optional_secure_value(value: String) -> Option<SecureValue> {
    optional_string(value).map(|value| SecureValue::Plain(SecretString::new(value.into())))
}

pub(super) fn append_digit(value: &mut u64, digit: char) -> bool {
    digit
        .to_digit(10)
        .and_then(|digit| value.checked_mul(10)?.checked_add(u64::from(digit)))
        .map(|next| {
            *value = next;
        })
        .is_some()
}

pub(super) fn pop_digit(value: &mut u64) {
    *value /= 10;
}

impl App {
    pub(super) fn replace_popup_kind(&mut self, kind: PopupType) {
        self.popup = Some(Popup::builder(kind).build());
    }
}
