//! Integration tests for flexbox layout engine using taffy.
//!
//! These tests verify the FlexLayoutEngine and FlexLayoutBuilder APIs
//! for creating CSS-like flexbox layouts in the TUI.

use ratatui::layout::Rect;
use splunk_tui::ui::layout::flex::{FlexLayoutBuilder, FlexLayoutEngine};
use taffy::prelude::*;

// taffy 0.5 uses `length` instead of `px` for pixel dimensions
use taffy::prelude::length as px;

// =============================================================================
// FlexLayoutEngine tests
// =============================================================================

#[test]
fn test_flex_layout_engine_creation() {
    let engine = FlexLayoutEngine::new();
    // Just verify it creates without panic
    let _ = engine.tree();
}

#[test]
fn test_flex_layout_engine_default() {
    let engine: FlexLayoutEngine = Default::default();
    let _ = engine.tree();
}

#[test]
fn test_simple_column_layout() {
    let mut engine = FlexLayoutEngine::new();

    // Create header (fixed height)
    let header = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: percent(100.0),
                height: px(3.0),
            },
            ..Default::default()
        })
        .unwrap();

    // Create main content (flex grow)
    let main = engine
        .new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: percent(100.0),
                height: auto(),
            },
            ..Default::default()
        })
        .unwrap();

    // Create footer (fixed height)
    let footer = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: percent(100.0),
                height: px(3.0),
            },
            ..Default::default()
        })
        .unwrap();

    // Create container
    let container = engine
        .new_with_children(
            taffy::Style {
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: px(100.0),
                    height: px(24.0),
                },
                ..Default::default()
            },
            &[header, main, footer],
        )
        .unwrap();

    // Compute layout
    engine.compute_layout(container, Size::MAX_CONTENT).unwrap();

    // Verify header rect
    let header_rect = engine.get_layout_rect(header, 0, 0).unwrap();
    assert_eq!(header_rect.height, 3);
    assert_eq!(header_rect.y, 0);

    // Verify footer rect
    let footer_rect = engine.get_layout_rect(footer, 0, 0).unwrap();
    assert_eq!(footer_rect.height, 3);
    assert_eq!(footer_rect.y, 21); // 24 - 3

    // Verify main takes remaining space
    let main_rect = engine.get_layout_rect(main, 0, 0).unwrap();
    assert_eq!(main_rect.height, 18); // 24 - 3 - 3
    assert_eq!(main_rect.y, 3); // Below header
}

#[test]
fn test_simple_row_layout() {
    let mut engine = FlexLayoutEngine::new();

    // Create sidebar (fixed width)
    let sidebar = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: px(30.0),
                height: percent(100.0),
            },
            ..Default::default()
        })
        .unwrap();

    // Create main content (flex grow)
    let main = engine
        .new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: percent(100.0),
            },
            ..Default::default()
        })
        .unwrap();

    // Create container
    let container = engine
        .new_with_children(
            taffy::Style {
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: px(100.0),
                    height: px(24.0),
                },
                ..Default::default()
            },
            &[sidebar, main],
        )
        .unwrap();

    // Compute layout
    engine.compute_layout(container, Size::MAX_CONTENT).unwrap();

    // Verify sidebar rect
    let sidebar_rect = engine.get_layout_rect(sidebar, 0, 0).unwrap();
    assert_eq!(sidebar_rect.width, 30);
    assert_eq!(sidebar_rect.x, 0);

    // Verify main rect
    let main_rect = engine.get_layout_rect(main, 0, 0).unwrap();
    assert_eq!(main_rect.width, 70); // 100 - 30
    assert_eq!(main_rect.x, 30); // To the right of sidebar
}

#[test]
fn test_compute_and_split() {
    let mut engine = FlexLayoutEngine::new();

    let header = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: percent(100.0),
                height: px(3.0),
            },
            ..Default::default()
        })
        .unwrap();

    let main = engine
        .new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: percent(100.0),
                height: auto(),
            },
            ..Default::default()
        })
        .unwrap();

    let container = engine
        .new_with_children(
            taffy::Style {
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: px(80.0),
                    height: px(24.0),
                },
                ..Default::default()
            },
            &[header, main],
        )
        .unwrap();

    let area = Rect::new(0, 0, 80, 24);
    let rects = engine.compute_and_split(container, area).unwrap();

    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].height, 3);
    assert_eq!(rects[1].height, 21);
    assert_eq!(rects[0].y, 0);
    assert_eq!(rects[1].y, 3);
}

#[test]
fn test_compute_and_split_with_offset() {
    let mut engine = FlexLayoutEngine::new();

    let left = engine
        .new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: percent(100.0),
            },
            ..Default::default()
        })
        .unwrap();

    let right = engine
        .new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: percent(100.0),
            },
            ..Default::default()
        })
        .unwrap();

    let container = engine
        .new_with_children(
            taffy::Style {
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: px(100.0),
                    height: px(24.0),
                },
                ..Default::default()
            },
            &[left, right],
        )
        .unwrap();

    let area = Rect::new(10, 5, 100, 24);
    let rects = engine.compute_and_split(container, area).unwrap();

    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].x, 10); // Parent offset applied
    assert_eq!(rects[0].y, 5);
    assert_eq!(rects[1].x, 60); // 10 + 50 (half of 100)
    assert_eq!(rects[1].y, 5);
}

// Note: Nested layouts with percentage-based sizing require careful handling
// of available space. When using nested containers, ensure that percentage
// values have a definite parent size to resolve against.

#[test]
fn test_add_and_remove_children() {
    let mut engine = FlexLayoutEngine::new();

    let parent = engine
        .new_leaf(taffy::Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .unwrap();

    let child1 = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: px(10.0),
                height: px(10.0),
            },
            ..Default::default()
        })
        .unwrap();

    let child2 = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: px(20.0),
                height: px(20.0),
            },
            ..Default::default()
        })
        .unwrap();

    // Add children
    engine.add_child(parent, child1).unwrap();
    engine.add_child(parent, child2).unwrap();

    let children: Vec<_> = engine.children(parent).unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0], child1);
    assert_eq!(children[1], child2);

    // Remove a child
    let removed = engine.remove_child(parent, child1).unwrap();
    assert_eq!(removed, child1);

    let children: Vec<_> = engine.children(parent).unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0], child2);
}

#[test]
fn test_clear_children() {
    let mut engine = FlexLayoutEngine::new();

    let parent = engine
        .new_leaf(taffy::Style {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .unwrap();

    let child1 = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: px(10.0),
                height: px(10.0),
            },
            ..Default::default()
        })
        .unwrap();

    let child2 = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: px(20.0),
                height: px(20.0),
            },
            ..Default::default()
        })
        .unwrap();

    engine.add_child(parent, child1).unwrap();
    engine.add_child(parent, child2).unwrap();

    assert_eq!(engine.children(parent).unwrap().len(), 2);

    engine.clear_children(parent).unwrap();

    assert!(engine.children(parent).unwrap().is_empty());
}

#[test]
fn test_set_style() {
    let mut engine = FlexLayoutEngine::new();

    let node = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: px(10.0),
                height: px(10.0),
            },
            ..Default::default()
        })
        .unwrap();

    // Update style
    engine
        .set_style(
            node,
            taffy::Style {
                size: Size {
                    width: px(50.0),
                    height: px(50.0),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let style = engine.style(node).unwrap();
    assert_eq!(style.size.width, px(50.0));
    assert_eq!(style.size.height, px(50.0));
}

// =============================================================================
// FlexLayoutBuilder tests
// =============================================================================

#[test]
fn test_flex_layout_builder_column() {
    let result = FlexLayoutBuilder::new()
        .column()
        .add_fixed_child(100.0, 3.0)
        .unwrap()
        .0
        .add_flex_child(1.0)
        .unwrap()
        .0
        .add_fixed_child(100.0, 3.0)
        .unwrap()
        .0
        .build();

    assert!(result.is_ok());
    let (engine, root) = result.unwrap();
    let children: Vec<_> = engine.children(root).unwrap();
    assert_eq!(children.len(), 3);
}

#[test]
fn test_flex_layout_builder_row() {
    let result = FlexLayoutBuilder::new()
        .row()
        .add_fixed_child(20.0, 100.0)
        .unwrap()
        .0
        .add_flex_child(1.0)
        .unwrap()
        .0
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_builder_without_root_fails() {
    let result = FlexLayoutBuilder::new().build();
    assert!(result.is_err());
}

#[test]
fn test_builder_add_percent_child() {
    let result = FlexLayoutBuilder::new()
        .column()
        .add_percent_child(100.0, 50.0)
        .unwrap()
        .0
        .add_percent_child(100.0, 50.0)
        .unwrap()
        .0
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_builder_add_child_with_style() {
    let result = FlexLayoutBuilder::new()
        .column()
        .add_child_with_style(taffy::Style {
            flex_grow: 2.0,
            size: Size {
                width: percent(100.0),
                height: auto(),
            },
            ..Default::default()
        })
        .unwrap()
        .0
        .build();

    assert!(result.is_ok());
}

// =============================================================================
// Helper function tests
// =============================================================================

// Note: The helper functions (header_main_footer, sidebar_main, equal_columns)
// create containers with percentage-based sizing. These are designed to be used
// as children of a parent with definite sizes. When using these helpers, wrap
// the returned container in a root node with definite dimensions.
//
// Example:
//   let (mut helper_engine, helper_container, ...) = header_main_footer(3.0, 3.0)?;
//   let root = engine.new_with_children(
//       Style { size: Size { width: length(100.0), height: length(24.0) }, ..Default::default() },
//       &[helper_container],
//   )?;
//   engine.compute_layout(root, Size::MAX_CONTENT)?;

// =============================================================================
// Edge case tests
// =============================================================================

#[test]
fn test_zero_size_container() {
    let mut engine = FlexLayoutEngine::new();

    let child = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: px(10.0),
                height: px(10.0),
            },
            ..Default::default()
        })
        .unwrap();

    let container = engine
        .new_with_children(
            taffy::Style {
                size: Size {
                    width: px(0.0),
                    height: px(0.0),
                },
                ..Default::default()
            },
            &[child],
        )
        .unwrap();

    // Should not panic
    engine
        .compute_layout(
            container,
            Size {
                width: AvailableSpace::Definite(0.0),
                height: AvailableSpace::Definite(0.0),
            },
        )
        .unwrap();
}

#[test]
fn test_flex_grow_distribution() {
    let mut engine = FlexLayoutEngine::new();

    // Create three children with different flex grow values
    let child1 = engine
        .new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: percent(100.0),
            },
            ..Default::default()
        })
        .unwrap();

    let child2 = engine
        .new_leaf(taffy::Style {
            flex_grow: 2.0,
            size: Size {
                width: auto(),
                height: percent(100.0),
            },
            ..Default::default()
        })
        .unwrap();

    let child3 = engine
        .new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: percent(100.0),
            },
            ..Default::default()
        })
        .unwrap();

    let container = engine
        .new_with_children(
            taffy::Style {
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: px(100.0),
                    height: px(24.0),
                },
                ..Default::default()
            },
            &[child1, child2, child3],
        )
        .unwrap();

    engine.compute_layout(container, Size::MAX_CONTENT).unwrap();

    let rect1 = engine.get_layout_rect(child1, 0, 0).unwrap();
    let rect2 = engine.get_layout_rect(child2, 0, 0).unwrap();
    let rect3 = engine.get_layout_rect(child3, 0, 0).unwrap();

    // Total flex grow = 4, so child1 = 25%, child2 = 50%, child3 = 25%
    assert_eq!(rect1.width, 25);
    assert_eq!(rect2.width, 50);
    assert_eq!(rect3.width, 25);
}

#[test]
fn test_mixed_fixed_and_flex() {
    let mut engine = FlexLayoutEngine::new();

    let fixed = engine
        .new_leaf(taffy::Style {
            size: Size {
                width: px(20.0),
                height: percent(100.0),
            },
            ..Default::default()
        })
        .unwrap();

    let flex = engine
        .new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: percent(100.0),
            },
            ..Default::default()
        })
        .unwrap();

    let container = engine
        .new_with_children(
            taffy::Style {
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: px(100.0),
                    height: px(24.0),
                },
                ..Default::default()
            },
            &[fixed, flex],
        )
        .unwrap();

    engine.compute_layout(container, Size::MAX_CONTENT).unwrap();

    let fixed_rect = engine.get_layout_rect(fixed, 0, 0).unwrap();
    let flex_rect = engine.get_layout_rect(flex, 0, 0).unwrap();

    assert_eq!(fixed_rect.width, 20);
    assert_eq!(flex_rect.width, 80); // 100 - 20
}
