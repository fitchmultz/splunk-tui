//! Reusable UI components for the TUI.
//!
//! This module provides generic, theme-aware components that can be used
//! across multiple screens for consistent UI patterns.
//!
//! # Available Components
//!
//! - [`SelectList<T>`]: Generic selectable list with keyboard navigation
//! - [`ScrollableContainer`]: Scrollable container for content that exceeds viewport
//! - [`DiffViewer`]: Unified diff viewer for comparing text
//! - [`LineNumberWidget`]: Line number gutter with diagnostic support
//! - [`Slider`]: Interactive slider for numeric input
//! - [`BigTextWidget`]: Large ASCII text headers
//! - [`AnimationController`]: Animation effects controller
//! - [`MarkdownRenderer`]: Markdown to text renderer
//!
//! # Usage
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::{
//!     SelectList, ScrollableContainer, DiffViewer, Slider, BigTextWidget
//! };
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

pub mod animation;
pub mod big_text;
pub mod diff_viewer;
pub mod line_numbers;
pub mod markdown;
pub mod scrollable;
pub mod select_list;
pub mod slider;

pub use animation::{AnimatedWidget, AnimationController, AnimationType, FadeIn, Opacity};
pub use big_text::{BigTextWidget, render_header, render_sub_header};
pub use diff_viewer::DiffViewer;
pub use line_numbers::{Diagnostic, DiagnosticSeverity, LineNumberWidget};
pub use markdown::{MarkdownRenderer, render_help_text, render_markdown};
pub use scrollable::ScrollableContainer;
pub use select_list::SelectList;
pub use slider::Slider;
