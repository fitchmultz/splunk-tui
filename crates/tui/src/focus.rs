//! Focus management for TUI components.
//!
//! Provides a `FocusManager` for tracking which component has focus
//! in multi-component screens, enabling keyboard navigation between
//! interactive elements.

use std::collections::HashMap;

/// Manages focus state for a collection of focusable components.
#[derive(Debug, Clone)]
pub struct FocusManager {
    /// Ordered list of component IDs that can receive focus.
    focus_order: Vec<String>,
    /// Index of the currently focused component in focus_order.
    current_focus: usize,
    /// Whether focus is enabled (can be disabled for read-only views).
    enabled: bool,
    /// Optional: track focus history for "back" navigation.
    focus_history: Vec<usize>,
    /// Component-specific data (e.g., scroll position).
    component_data: HashMap<String, ComponentFocusData>,
}

/// Per-component focus data.
#[derive(Debug, Clone, Default)]
pub struct ComponentFocusData {
    /// Scroll offset when component was last focused.
    pub scroll_offset: usize,
    /// Cursor position when component was last focused.
    pub cursor_position: usize,
    /// Custom data storage.
    pub custom: Option<String>,
}

impl FocusManager {
    /// Create a new focus manager with the given component IDs.
    pub fn new(component_ids: Vec<String>) -> Self {
        Self {
            focus_order: component_ids,
            current_focus: 0,
            enabled: true,
            focus_history: Vec::new(),
            component_data: HashMap::new(),
        }
    }

    /// Create a new focus manager from string slices.
    pub fn from_ids(ids: &[&str]) -> Self {
        Self::new(ids.iter().map(|s| s.to_string()).collect())
    }

    /// Move focus to the next component.
    pub fn next(&mut self) {
        if !self.enabled || self.focus_order.is_empty() {
            return;
        }
        self.focus_history.push(self.current_focus);
        self.current_focus = (self.current_focus + 1) % self.focus_order.len();
    }

    /// Move focus to the previous component.
    pub fn prev(&mut self) {
        if !self.enabled || self.focus_order.is_empty() {
            return;
        }
        self.focus_history.push(self.current_focus);
        if self.current_focus == 0 {
            self.current_focus = self.focus_order.len() - 1;
        } else {
            self.current_focus -= 1;
        }
    }

    /// Check if a specific component ID is currently focused.
    pub fn is_focused(&self, id: &str) -> bool {
        if !self.enabled {
            return false;
        }
        self.focus_order
            .get(self.current_focus)
            .map(|current| current == id)
            .unwrap_or(false)
    }

    /// Get the ID of the currently focused component.
    pub fn current_id(&self) -> Option<&str> {
        if !self.enabled {
            return None;
        }
        self.focus_order.get(self.current_focus).map(|s| s.as_str())
    }

    /// Set focus to a specific component by ID.
    pub fn set_focus(&mut self, id: &str) -> bool {
        if let Some(index) = self.focus_order.iter().position(|x| x == id) {
            self.focus_history.push(self.current_focus);
            self.current_focus = index;
            true
        } else {
            false
        }
    }

    /// Enable focus management.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable focus management (no component appears focused).
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if focus management is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Add a new focusable component.
    pub fn add_component(&mut self, id: String) {
        if !self.focus_order.contains(&id) {
            self.focus_order.push(id);
        }
    }

    /// Remove a focusable component.
    pub fn remove_component(&mut self, id: &str) {
        if let Some(index) = self.focus_order.iter().position(|x| x == id) {
            self.focus_order.remove(index);

            // Adjust current_focus based on removal position
            if index < self.current_focus {
                // Removed component was before current focus, decrement to maintain focus
                self.current_focus -= 1;
            } else if self.current_focus >= self.focus_order.len() && self.current_focus > 0 {
                // Removed at/after current focus but current_focus is now out of bounds
                self.current_focus -= 1;
            }
        }
    }

    /// Store data for a component.
    pub fn store_data(&mut self, id: &str, data: ComponentFocusData) {
        self.component_data.insert(id.to_string(), data);
    }

    /// Retrieve data for a component.
    pub fn get_data(&self, id: &str) -> Option<&ComponentFocusData> {
        self.component_data.get(id)
    }

    /// Get mutable data for a component.
    pub fn get_data_mut(&mut self, id: &str) -> Option<&mut ComponentFocusData> {
        self.component_data.get_mut(id)
    }

    /// Navigate back to previous focus (if history exists).
    pub fn back(&mut self) -> bool {
        if let Some(previous) = self.focus_history.pop() {
            self.current_focus = previous;
            true
        } else {
            false
        }
    }

    /// Get the number of focusable components.
    pub fn len(&self) -> usize {
        self.focus_order.len()
    }

    /// Check if there are any focusable components.
    pub fn is_empty(&self) -> bool {
        self.focus_order.is_empty()
    }

    /// Get all component IDs.
    pub fn component_ids(&self) -> &[String] {
        &self.focus_order
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self {
            focus_order: Vec::new(),
            current_focus: 0,
            enabled: true,
            focus_history: Vec::new(),
            component_data: HashMap::new(),
        }
    }
}

/// Trait for components that can receive focus.
pub trait Focusable {
    /// Get the unique ID of this component.
    fn id(&self) -> &str;

    /// Called when this component receives focus.
    fn on_focus(&mut self);

    /// Called when this component loses focus.
    fn on_blur(&mut self);

    /// Check if this component is currently focusable.
    fn can_focus(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_manager_creation() {
        let fm = FocusManager::from_ids(&["input", "list", "button"]);
        assert_eq!(fm.len(), 3);
        assert!(fm.is_focused("input"));
    }

    #[test]
    fn test_focus_navigation() {
        let mut fm = FocusManager::from_ids(&["a", "b", "c"]);

        assert!(fm.is_focused("a"));

        fm.next();
        assert!(fm.is_focused("b"));

        fm.next();
        assert!(fm.is_focused("c"));

        fm.next();
        assert!(fm.is_focused("a")); // Wrap around

        fm.prev();
        assert!(fm.is_focused("c")); // Wrap backward
    }

    #[test]
    fn test_set_focus() {
        let mut fm = FocusManager::from_ids(&["a", "b", "c"]);

        assert!(fm.set_focus("c"));
        assert!(fm.is_focused("c"));

        assert!(!fm.set_focus("nonexistent"));
    }

    #[test]
    fn test_disabled_focus() {
        let mut fm = FocusManager::from_ids(&["a", "b"]);
        fm.disable();

        assert!(!fm.is_focused("a"));
        assert!(fm.current_id().is_none());

        fm.enable();
        assert!(fm.is_focused("a"));
    }

    #[test]
    fn test_component_data() {
        let mut fm = FocusManager::from_ids(&["list"]);

        fm.store_data(
            "list",
            ComponentFocusData {
                scroll_offset: 42,
                cursor_position: 5,
                custom: None,
            },
        );

        let data = fm.get_data("list").unwrap();
        assert_eq!(data.scroll_offset, 42);
        assert_eq!(data.cursor_position, 5);
    }

    #[test]
    fn test_add_remove_component() {
        let mut fm = FocusManager::from_ids(&["a", "b"]);

        fm.add_component("c".to_string());
        assert_eq!(fm.len(), 3);

        fm.remove_component("b");
        assert_eq!(fm.len(), 2);
        assert!(!fm.component_ids().contains(&"b".to_string()));
    }

    #[test]
    fn test_remove_current_focus_adjustment() {
        let mut fm = FocusManager::from_ids(&["a", "b", "c"]);

        // Focus on "c" (index 2)
        fm.set_focus("c");
        assert!(fm.is_focused("c"));

        // Remove "b" (index 1), "c" should still be focused (now at index 1)
        fm.remove_component("b");
        assert!(fm.is_focused("c"));
        assert_eq!(fm.len(), 2);
    }

    #[test]
    fn test_focus_history() {
        let mut fm = FocusManager::from_ids(&["a", "b", "c"]);

        fm.next(); // a -> b
        fm.next(); // b -> c

        assert!(fm.back()); // c -> b
        assert!(fm.is_focused("b"));

        assert!(fm.back()); // b -> a
        assert!(fm.is_focused("a"));

        assert!(!fm.back()); // No more history
    }

    #[test]
    fn test_empty_focus_manager() {
        let fm = FocusManager::default();

        assert!(fm.is_empty());
        assert_eq!(fm.len(), 0);
        assert_eq!(fm.current_id(), None);
        assert!(!fm.is_focused("anything"));
    }

    #[test]
    fn test_navigation_with_empty_manager() {
        let mut fm = FocusManager::default();

        // Should not panic
        fm.next();
        fm.prev();
        fm.set_focus("anything");
    }

    #[test]
    fn test_current_id_returns_none_when_disabled() {
        let mut fm = FocusManager::from_ids(&["a", "b"]);
        fm.disable();

        assert_eq!(fm.current_id(), None);
    }

    #[test]
    fn test_component_ids_returns_all_ids() {
        let fm = FocusManager::from_ids(&["a", "b", "c"]);

        let ids = fm.component_ids();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"a".to_string()));
        assert!(ids.contains(&"b".to_string()));
        assert!(ids.contains(&"c".to_string()));
    }

    #[test]
    fn test_get_data_mut() {
        let mut fm = FocusManager::from_ids(&["list"]);

        fm.store_data(
            "list",
            ComponentFocusData {
                scroll_offset: 10,
                cursor_position: 5,
                custom: None,
            },
        );

        // Modify via mutable reference
        if let Some(data) = fm.get_data_mut("list") {
            data.scroll_offset = 20;
        }

        let data = fm.get_data("list").unwrap();
        assert_eq!(data.scroll_offset, 20);
    }

    #[test]
    fn test_add_duplicate_component_ignored() {
        let mut fm = FocusManager::from_ids(&["a", "b"]);

        // Adding duplicate should not increase count
        fm.add_component("a".to_string());
        assert_eq!(fm.len(), 2);
    }

    #[test]
    fn test_remove_last_component_adjusts_focus() {
        let mut fm = FocusManager::from_ids(&["a", "b"]);

        // Focus on "b" (index 1)
        fm.set_focus("b");
        assert!(fm.is_focused("b"));

        // Remove "a", "b" should still be focused (now at index 0)
        fm.remove_component("a");
        assert!(fm.is_focused("b"));
        assert_eq!(fm.current_focus, 0);
    }

    #[test]
    fn test_remove_all_components() {
        let mut fm = FocusManager::from_ids(&["a", "b"]);

        fm.remove_component("a");
        fm.remove_component("b");

        assert!(fm.is_empty());
        assert_eq!(fm.current_focus, 0); // Should not underflow
    }

    #[test]
    fn test_remove_component_before_current_focus() {
        let mut fm = FocusManager::from_ids(&["a", "b", "c"]);
        fm.set_focus("b"); // current_focus = 1
        assert!(fm.is_focused("b"));

        // Remove "a" at index 0 (before current_focus)
        fm.remove_component("a");

        // "b" should still be focused, now at index 0
        assert!(fm.is_focused("b"));
        assert_eq!(fm.current_focus, 0);
        assert_eq!(fm.len(), 2);
    }

    #[test]
    fn test_remove_focused_component_prefers_next() {
        let mut fm = FocusManager::from_ids(&["a", "b", "c"]);
        fm.set_focus("b"); // current_focus = 1, focused on "b"

        // Remove the currently focused component "b"
        // The new component at index 1 is "c", so focus stays at index 1
        fm.remove_component("b");

        // Should focus on "c" (next) since it shifted to current index
        assert!(fm.is_focused("c"));
        assert_eq!(fm.current_focus, 1);
        assert_eq!(fm.len(), 2);
    }

    #[test]
    fn test_remove_component_after_current_focus() {
        let mut fm = FocusManager::from_ids(&["a", "b", "c"]);
        fm.set_focus("b"); // current_focus = 1
        assert!(fm.is_focused("b"));

        // Remove "c" at index 2 (after current_focus)
        fm.remove_component("c");

        // "b" should still be focused at index 1
        assert!(fm.is_focused("b"));
        assert_eq!(fm.current_focus, 1);
        assert_eq!(fm.len(), 2);
    }

    #[test]
    fn test_remove_focused_component_at_index_zero() {
        let mut fm = FocusManager::from_ids(&["a", "b", "c"]);
        // current_focus starts at 0, focused on "a"
        assert!(fm.is_focused("a"));

        // Remove the currently focused component "a"
        // "b" shifts to index 0, focus stays at index 0
        fm.remove_component("a");

        // Should focus on "b" (now at index 0)
        assert!(fm.is_focused("b"));
        assert_eq!(fm.current_focus, 0);
        assert_eq!(fm.len(), 2);
    }

    #[test]
    fn test_remove_focused_last_component() {
        let mut fm = FocusManager::from_ids(&["a", "b", "c"]);
        fm.set_focus("c"); // current_focus = 2, focused on "c"

        // Remove the last focused component "c"
        fm.remove_component("c");

        // Should focus on "b" (previous, now at index 1)
        assert!(fm.is_focused("b"));
        assert_eq!(fm.current_focus, 1);
        assert_eq!(fm.len(), 2);
    }
}
