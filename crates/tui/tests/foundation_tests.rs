//! Integration tests for TUI foundation modules.
//!
//! Tests the theme helpers, focus management, and component library.

use splunk_config::{ColorTheme, Theme};
use splunk_tui::{
    focus::{ComponentFocusData, FocusManager},
    ui::theme::ThemeExt,
};

// Theme Extension Tests

#[test]
fn test_theme_ext_styles() {
    let theme = Theme::from_color_theme(ColorTheme::Default);

    // Test basic styles
    let text_style = theme.text();
    assert_eq!(text_style.fg, Some(theme.text));

    let title_style = theme.title();
    assert_eq!(title_style.fg, Some(theme.accent));

    let border_style = theme.border();
    assert_eq!(border_style.fg, Some(theme.border));
}

#[test]
fn test_theme_ext_semantic_styles() {
    let theme = Theme::from_color_theme(ColorTheme::Default);

    assert_eq!(theme.success().fg, Some(theme.success));
    assert_eq!(theme.warning().fg, Some(theme.warning));
    assert_eq!(theme.error().fg, Some(theme.error));
    assert_eq!(theme.info().fg, Some(theme.info));
}

#[test]
fn test_theme_ext_different_themes() {
    // Test that theme helpers work across all themes
    for color_theme in [
        ColorTheme::Default,
        ColorTheme::Light,
        ColorTheme::Dark,
        ColorTheme::HighContrast,
    ] {
        let theme = Theme::from_color_theme(color_theme);

        // All themes should provide consistent style helpers
        let _ = theme.text();
        let _ = theme.title();
        let _ = theme.border();
        let _ = theme.border_focused();
        let _ = theme.highlight();
        let _ = theme.success();
        let _ = theme.warning();
        let _ = theme.error();
        let _ = theme.info();
        let _ = theme.disabled();
        let _ = theme.table_header();
        let _ = theme.syntax_command();
        let _ = theme.syntax_string();
        let _ = theme.syntax_number();
        let _ = theme.syntax_comment();
    }
}

// Focus Manager Tests

#[test]
fn test_focus_manager_integration() {
    let mut fm = FocusManager::from_ids(&["search", "results", "details"]);

    // Initial focus
    assert!(fm.is_focused("search"));
    assert_eq!(fm.current_id(), Some("search"));

    // Navigation
    fm.next();
    assert!(fm.is_focused("results"));

    fm.next();
    assert!(fm.is_focused("details"));

    // Wrap around
    fm.next();
    assert!(fm.is_focused("search"));
}

#[test]
fn test_focus_manager_data_storage() {
    let mut fm = FocusManager::from_ids(&["list"]);

    fm.store_data(
        "list",
        ComponentFocusData {
            scroll_offset: 100,
            cursor_position: 50,
            custom: Some("test".to_string()),
        },
    );

    let data = fm.get_data("list").unwrap();
    assert_eq!(data.scroll_offset, 100);
    assert_eq!(data.cursor_position, 50);
    assert_eq!(data.custom, Some("test".to_string()));
}

#[test]
fn test_focus_manager_enable_disable() {
    let mut fm = FocusManager::from_ids(&["a", "b"]);

    assert!(fm.is_enabled());
    assert!(fm.is_focused("a"));

    fm.disable();
    assert!(!fm.is_enabled());
    assert!(!fm.is_focused("a"));
    assert_eq!(fm.current_id(), None);

    fm.enable();
    assert!(fm.is_enabled());
    assert!(fm.is_focused("a"));
}

#[test]
fn test_focus_manager_empty() {
    let fm = FocusManager::default();

    assert!(fm.is_empty());
    assert_eq!(fm.len(), 0);
    assert_eq!(fm.current_id(), None);
}

#[test]
fn test_focus_manager_set_focus() {
    let mut fm = FocusManager::from_ids(&["a", "b", "c"]);

    assert!(fm.set_focus("c"));
    assert!(fm.is_focused("c"));

    assert!(!fm.set_focus("nonexistent"));
}

// Component Integration Tests

#[test]
fn test_select_list_with_theme() {
    use splunk_tui::ui::components::SelectList;

    let items = vec!["Item 1", "Item 2", "Item 3"];
    let list = SelectList::new(items);

    assert_eq!(list.len(), 3);
    assert_eq!(list.selected_index(), 0);
    assert_eq!(list.selected(), Some(&"Item 1"));
}

#[test]
fn test_scrollable_container_bounds() {
    use splunk_tui::ui::components::ScrollableContainer;

    let mut container = ScrollableContainer::new(100);

    // Test that scroll stays within bounds
    container.scroll_down(50);
    assert_eq!(container.scroll_offset(), 50);

    container.scroll_down(100);
    assert_eq!(container.scroll_offset(), 99); // Max scroll

    container.scroll_up(200);
    assert_eq!(container.scroll_offset(), 0); // Min scroll
}

#[test]
fn test_select_list_navigation() {
    use splunk_tui::ui::components::SelectList;

    let items: Vec<String> = (0..20).map(|i| format!("item {}", i)).collect();
    let mut list = SelectList::new(items);

    assert_eq!(list.selected_index(), 0);

    list.next();
    assert_eq!(list.selected_index(), 1);

    list.prev();
    assert_eq!(list.selected_index(), 0);

    // Test page down
    list.page_down();
    assert_eq!(list.selected_index(), 9);

    // Test last
    list.last();
    assert_eq!(list.selected_index(), 19);

    // Test first
    list.first();
    assert_eq!(list.selected_index(), 0);
}

#[test]
fn test_select_list_with_formatter() {
    use splunk_tui::ui::components::SelectList;

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

    let list = SelectList::with_formatter(items, |item| format!("{}: {}", item.name, item.value));

    assert_eq!(list.len(), 2);
}

#[test]
fn test_scrollable_container_navigation() {
    use splunk_tui::ui::components::ScrollableContainer;

    let mut container = ScrollableContainer::new(100);

    assert_eq!(container.scroll_offset(), 0);

    container.scroll_down(10);
    assert_eq!(container.scroll_offset(), 10);

    container.scroll_up(5);
    assert_eq!(container.scroll_offset(), 5);

    container.scroll_to_bottom();
    assert_eq!(container.scroll_offset(), 99);

    container.scroll_to_top();
    assert_eq!(container.scroll_offset(), 0);
}

#[test]
fn test_scrollable_container_page_navigation() {
    use splunk_tui::ui::components::ScrollableContainer;

    let mut container = ScrollableContainer::new(100);

    container.page_down(10);
    assert_eq!(container.scroll_offset(), 9);

    container.page_down(10);
    assert_eq!(container.scroll_offset(), 18);

    container.page_up(10);
    assert_eq!(container.scroll_offset(), 9);
}

#[test]
fn test_scrollable_container_visible_range() {
    use splunk_tui::ui::components::ScrollableContainer;

    let mut container = ScrollableContainer::new(100);
    container.scroll_down(20);

    let range = container.visible_range(10);
    assert_eq!(range.start, 20);
    assert_eq!(range.end, 30);
}

#[test]
fn test_focus_manager_history_navigation() {
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
fn test_theme_ext_highlight_style() {
    let theme = Theme::from_color_theme(ColorTheme::Default);

    let highlight = theme.highlight();
    assert_eq!(highlight.fg, Some(theme.highlight_fg));
    assert_eq!(highlight.bg, Some(theme.highlight_bg));
}

#[test]
fn test_theme_ext_table_header_style() {
    let theme = Theme::from_color_theme(ColorTheme::Default);

    let header = theme.table_header();
    assert_eq!(header.fg, Some(theme.table_header_fg));
    assert_eq!(header.bg, Some(theme.table_header_bg));
}

#[test]
fn test_select_list_focus_state() {
    use splunk_tui::ui::components::SelectList;

    let items = vec!["a", "b"];
    let mut list = SelectList::new(items);

    assert!(!list.is_focused());

    list.set_focused(true);
    assert!(list.is_focused());
}

#[test]
fn test_scrollable_container_focus_state() {
    use splunk_tui::ui::components::ScrollableContainer;

    let mut container = ScrollableContainer::new(100);

    assert!(!container.is_focused());

    container.set_focused(true);
    assert!(container.is_focused());
}

#[test]
fn test_component_data_mutability() {
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
fn test_select_list_set_items_adjusts_selection() {
    use splunk_tui::ui::components::SelectList;

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
fn test_scrollable_container_set_total_lines_adjusts_scroll() {
    use splunk_tui::ui::components::ScrollableContainer;

    let mut container = ScrollableContainer::new(100);
    container.scroll_to_bottom();
    assert_eq!(container.scroll_offset(), 99);

    // Reduce total lines, scroll should adjust
    container.set_total_lines(50);
    assert_eq!(container.scroll_offset(), 49);
}

#[test]
fn test_focus_manager_add_remove_component() {
    let mut fm = FocusManager::from_ids(&["a", "b"]);

    fm.add_component("c".to_string());
    assert_eq!(fm.len(), 3);

    fm.remove_component("b");
    assert_eq!(fm.len(), 2);
    assert!(!fm.component_ids().contains(&"b".to_string()));
}

#[test]
fn test_theme_ext_syntax_styles() {
    let theme = Theme::from_color_theme(ColorTheme::Default);

    let command = theme.syntax_command();
    let string = theme.syntax_string();
    let number = theme.syntax_number();
    let comment = theme.syntax_comment();

    assert_eq!(command.fg, Some(theme.syntax_command));
    assert_eq!(string.fg, Some(theme.syntax_string));
    assert_eq!(number.fg, Some(theme.syntax_number));
    assert_eq!(comment.fg, Some(theme.syntax_comment));
}

#[test]
fn test_select_list_empty_navigation() {
    use splunk_tui::ui::components::SelectList;

    let mut list: SelectList<String> = SelectList::new(Vec::new());

    // Should not panic on empty list
    list.next();
    list.prev();
    list.page_down();
    list.page_up();

    assert!(list.selected().is_none());
    assert!(list.is_empty());
}

#[test]
fn test_scrollable_container_empty() {
    use splunk_tui::ui::components::ScrollableContainer;

    let container = ScrollableContainer::new(0);

    assert_eq!(container.max_scroll(), 0);
    assert_eq!(container.scroll_offset(), 0);

    let range = container.visible_range(10);
    assert!(range.is_empty());
}

#[test]
fn test_theme_ext_border_focused() {
    let theme = Theme::from_color_theme(ColorTheme::Default);

    let focused = theme.border_focused();
    assert_eq!(focused.fg, Some(theme.accent));
}

#[test]
fn test_theme_ext_disabled() {
    let theme = Theme::from_color_theme(ColorTheme::Default);

    let disabled = theme.disabled();
    assert_eq!(disabled.fg, Some(theme.disabled));
}

#[test]
fn test_focus_manager_prev_wrap() {
    let mut fm = FocusManager::from_ids(&["a", "b", "c"]);

    // At "a", prev should wrap to "c"
    fm.prev();
    assert!(fm.is_focused("c"));

    // At "c", prev should go to "b"
    fm.prev();
    assert!(fm.is_focused("b"));
}

#[test]
fn test_select_list_state() {
    use splunk_tui::ui::components::SelectList;

    let items = vec!["a", "b", "c"];
    let mut list = SelectList::new(items);
    list.set_selected(1);

    let state = list.state();
    assert_eq!(state.selected(), Some(1));
}

#[test]
fn test_scrollable_container_is_visible() {
    use splunk_tui::ui::components::ScrollableContainer;

    let container = ScrollableContainer::new(100);

    assert!(container.is_visible(0, 10));
    assert!(container.is_visible(9, 10));
    assert!(!container.is_visible(10, 10));
}

#[test]
fn test_theme_ext_text_dim() {
    let theme = Theme::from_color_theme(ColorTheme::Default);

    let dim = theme.text_dim();
    assert_eq!(dim.fg, Some(theme.text_dim));
}
