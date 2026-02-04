//! Generic selectable list component.
//!
//! A reusable list component with selection, scrolling, and theme-aware rendering.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::SelectList;
//!
//! let items = vec!["Option 1", "Option 2", "Option 3"];
//! let mut list = SelectList::new(items);
//!
//! // Navigate
//! list.next();
//! list.prev();
//!
//! // Get selected item
//! if let Some(item) = list.selected() {
//!     println!("Selected: {}", item);
//! }
//! ```

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, List, ListItem, ListState},
};
use splunk_config::Theme;
use std::fmt::Display;

use crate::ui::theme::ThemeExt;

/// A generic selectable list component.
#[derive(Debug, Clone)]
pub struct SelectList<T> {
    /// Items in the list.
    items: Vec<T>,
    /// Currently selected index.
    selected: usize,
    /// Scroll offset (first visible item).
    scroll_offset: usize,
    /// Number of visible items.
    visible_count: usize,
    /// Optional block (borders/title).
    block: Option<Block<'static>>,
    /// Whether the list is focused.
    focused: bool,
    /// Custom item formatter.
    formatter: Option<fn(&T) -> String>,
}

impl<T: Display> SelectList<T> {
    /// Create a new select list with the given items.
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            selected: 0,
            scroll_offset: 0,
            visible_count: 10,
            block: None,
            focused: false,
            formatter: None,
        }
    }

    /// Create a new select list with a custom formatter.
    pub fn with_formatter(items: Vec<T>, formatter: fn(&T) -> String) -> Self {
        Self {
            items,
            selected: 0,
            scroll_offset: 0,
            visible_count: 10,
            block: None,
            focused: false,
            formatter: Some(formatter),
        }
    }

    /// Set the block (borders/title) for the list.
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the visible count.
    pub fn visible_count(mut self, count: usize) -> Self {
        self.visible_count = count;
        self
    }

    /// Set whether the list is focused.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Check if the list is focused.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Move selection to the next item.
    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.items.len() - 1);
        self.adjust_scroll();
    }

    /// Move selection to the previous item.
    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.adjust_scroll();
    }

    /// Move selection to the first item.
    pub fn first(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Move selection to the last item.
    pub fn last(&mut self) {
        if !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
        self.adjust_scroll();
    }

    /// Move selection down by page size.
    pub fn page_down(&mut self) {
        let page_size = self.visible_count.saturating_sub(1);
        self.selected = (self.selected + page_size).min(self.items.len().saturating_sub(1));
        self.adjust_scroll();
    }

    /// Move selection up by page size.
    pub fn page_up(&mut self) {
        let page_size = self.visible_count.saturating_sub(1);
        self.selected = self.selected.saturating_sub(page_size);
        self.adjust_scroll();
    }

    /// Adjust scroll offset to keep selected item visible.
    fn adjust_scroll(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.visible_count {
            self.scroll_offset = self.selected.saturating_sub(self.visible_count - 1);
        }
    }

    /// Get the currently selected item.
    pub fn selected(&self) -> Option<&T> {
        self.items.get(self.selected)
    }

    /// Get the currently selected index.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Set the selected index.
    pub fn set_selected(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected = index;
            self.adjust_scroll();
        }
    }

    /// Get all items.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Get mutable access to items.
    pub fn items_mut(&mut self) -> &mut Vec<T> {
        &mut self.items
    }

    /// Replace all items.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
        self.adjust_scroll();
    }

    /// Get the number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Format an item for display.
    fn format_item(&self, item: &T) -> String {
        match self.formatter {
            Some(f) => f(item),
            None => item.to_string(),
        }
    }

    /// Render the list.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let visible_items: Vec<ListItem> = self
            .items
            .iter()
            .skip(self.scroll_offset)
            .take(self.visible_count)
            .enumerate()
            .map(|(i, item)| {
                let actual_index = i + self.scroll_offset;
                let is_selected = actual_index == self.selected;

                let style = if is_selected {
                    theme.highlight().add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };

                let prefix = if is_selected { "> " } else { "  " };
                let text = format!("{}{}", prefix, self.format_item(item));

                ListItem::new(Line::from(text)).style(style)
            })
            .collect();

        let mut list = List::new(visible_items);

        if let Some(block) = &self.block {
            let border_style = if self.focused {
                theme.border_focused()
            } else {
                theme.border()
            };
            list = list.block(block.clone().border_style(border_style));
        }

        frame.render_widget(list, area);
    }

    /// Get a ListState for use with ratatui's StatefulWidget.
    pub fn state(&self) -> ListState {
        let mut state = ListState::default();
        state.select(Some(self.selected));
        state
    }
}

impl<T: Display> Default for SelectList<T> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_list_creation() {
        let items = vec!["a", "b", "c"];
        let list = SelectList::new(items);

        assert_eq!(list.len(), 3);
        assert_eq!(list.selected_index(), 0);
        assert_eq!(list.selected(), Some(&"a"));
    }

    #[test]
    fn test_select_list_navigation() {
        let items: Vec<String> = (0..20).map(|i| format!("item {}", i)).collect();
        let mut list = SelectList::new(items);
        list.visible_count = 5;

        assert_eq!(list.selected_index(), 0);

        list.next();
        assert_eq!(list.selected_index(), 1);

        list.prev();
        assert_eq!(list.selected_index(), 0);

        // Test page down
        list.page_down();
        assert_eq!(list.selected_index(), 4);

        // Test last
        list.last();
        assert_eq!(list.selected_index(), 19);

        // Test first
        list.first();
        assert_eq!(list.selected_index(), 0);
    }

    #[test]
    fn test_select_list_scroll_adjustment() {
        let items: Vec<String> = (0..20).map(|i| format!("item {}", i)).collect();
        let mut list = SelectList::new(items);
        list.visible_count = 5;

        // Select item 10, should adjust scroll
        list.set_selected(10);
        assert_eq!(list.selected_index(), 10);
        assert_eq!(list.scroll_offset, 6); // 10 - (5 - 1) = 6
    }

    #[test]
    fn test_select_list_empty() {
        let mut list: SelectList<String> = SelectList::new(Vec::new());

        list.next(); // Should not panic
        list.prev(); // Should not panic

        assert!(list.selected().is_none());
        assert!(list.is_empty());
    }

    #[test]
    fn test_select_list_with_formatter() {
        #[derive(Debug)]
        struct Item {
            name: String,
            value: i32,
        }

        impl std::fmt::Display for Item {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}: {}", self.name, self.value)
            }
        }

        let items = vec![
            Item {
                name: "a".to_string(),
                value: 1,
            },
            Item {
                name: "b".to_string(),
                value: 2,
            },
        ];

        let list =
            SelectList::with_formatter(items, |item| format!("{}: {}", item.name, item.value));

        assert_eq!(
            list.format_item(&Item {
                name: "x".to_string(),
                value: 42
            }),
            "x: 42"
        );
    }

    #[test]
    fn test_select_list_set_items() {
        let items = vec!["a", "b", "c"];
        let mut list = SelectList::new(items);

        list.set_selected(2);
        assert_eq!(list.selected_index(), 2);

        // Replace with fewer items - selection should adjust
        list.set_items(vec!["x", "y"]);
        assert_eq!(list.len(), 2);
        assert_eq!(list.selected_index(), 1); // Adjusted from 2 to 1
    }

    #[test]
    fn test_select_list_focus() {
        let items = vec!["a", "b"];
        let mut list = SelectList::new(items);

        assert!(!list.is_focused());

        list.set_focused(true);
        assert!(list.is_focused());

        list.set_focused(false);
        assert!(!list.is_focused());
    }

    #[test]
    fn test_select_list_state() {
        let items = vec!["a", "b", "c"];
        let mut list = SelectList::new(items);
        list.set_selected(1);

        let state = list.state();
        assert_eq!(state.selected(), Some(1));
    }

    #[test]
    fn test_select_list_page_navigation() {
        let items: Vec<String> = (0..100).map(|i| format!("item {}", i)).collect();
        let mut list = SelectList::new(items);
        list.visible_count = 10;

        // Page down from 0 should go to 9
        list.page_down();
        assert_eq!(list.selected_index(), 9);

        // Page down again should go to 18
        list.page_down();
        assert_eq!(list.selected_index(), 18);

        // Page up should go back to 9
        list.page_up();
        assert_eq!(list.selected_index(), 9);
    }

    #[test]
    fn test_select_list_items_mut() {
        let items = vec!["a", "b", "c"];
        let mut list = SelectList::new(items);

        // Modify items through mutable reference
        list.items_mut().push("d");
        assert_eq!(list.len(), 4);
    }

    #[test]
    fn test_select_list_items_accessor() {
        let items = vec!["a", "b", "c"];
        let list = SelectList::new(items);

        let items_ref = list.items();
        assert_eq!(items_ref.len(), 3);
        assert_eq!(items_ref[0], "a");
    }
}
