//! Reusable UI components for the TUI.
//!
//! This module provides generic, theme-aware components that can be used
//! across multiple screens for consistent UI patterns.
//!
//! # Available Components
//!
//! - [`SelectList<T>`]: Generic selectable list with keyboard navigation
//! - [`ScrollableContainer`]: Scrollable container for content that exceeds viewport
//!
//! # Usage
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::{SelectList, ScrollableContainer};
//!
//! // Create a select list
//! let items = vec!["Item 1", "Item 2", "Item 3"];
//! let mut list = SelectList::new(items);
//!
//! // Navigate
//! list.next();
//! list.prev();
//!
//! // Get selection
//! if let Some(selected) = list.selected() {
//!     println!("Selected: {}", selected);
//! }
//! ```

pub mod scrollable;
pub mod select_list;

pub use scrollable::ScrollableContainer;
pub use select_list::SelectList;
