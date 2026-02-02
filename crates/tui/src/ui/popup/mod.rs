//! Modal popup rendering for confirmations and help.
//!
//! This module provides a Builder pattern for constructing popups with
//! customizable titles, content, and types. Popups are rendered as
//! centered modal dialogs overlaid on the main UI.

mod builder;
mod profile_field;
mod render;
mod saved_search_field;
mod types;

/// Default popup dimensions as percentages of screen size.
pub const POPUP_WIDTH_PERCENT: u16 = 60;
pub const POPUP_HEIGHT_PERCENT: u16 = 50;

// Re-export public types for backward compatibility
pub use builder::{Popup, PopupBuilder};
pub use profile_field::ProfileField;
pub use render::render_popup;
pub use saved_search_field::SavedSearchField;
pub use types::PopupType;
