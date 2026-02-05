//! Flexbox layout engine using taffy (Yoga-based)
//!
//! Provides CSS-like flexbox layouts for ratatui components.
//!
//! This module wraps the `taffy` crate (which uses the Yoga layout engine,
//! same as OpenTUI) to provide flexbox layout capabilities for the TUI.

use ratatui::layout::Rect;
use taffy::prelude::*;

/// Re-export taffy dimension helpers for convenience.
/// Note: In taffy 0.5, `px` is called `length`.
pub use taffy::prelude::{auto, length, percent};

/// Wrapper around TaffyTree for integration with ratatui.
///
/// This provides a higher-level API for creating flexbox layouts
/// that translate to ratatui `Rect` areas.
///
/// # Example
///
/// ```ignore
/// use splunk_tui::ui::layout::flex::FlexLayoutEngine;
/// use splunk_tui::ui::layout::flex::{auto, length, percent};
/// use ratatui::layout::Rect;
/// use taffy::{Size, FlexDirection};
///
/// let mut engine = FlexLayoutEngine::new();
///
/// // Create a header with fixed height
/// let header = engine.new_leaf(taffy::Style {
///     size: Size { width: percent(100.0), height: length(3.0) },
///     ..Default::default()
/// }).unwrap();
///
/// // Create main content that grows to fill space
/// let main = engine.new_leaf(taffy::Style {
///     flex_grow: 1.0,
///     size: Size { width: percent(100.0), height: auto() },
///     ..Default::default()
/// }).unwrap();
///
/// // Create container with column layout
/// let container = engine.new_with_children(
///     taffy::Style {
///         flex_direction: FlexDirection::Column,
///         size: Size { width: length(100.0), height: length(24.0) },
///         ..Default::default()
///     },
///     &[header, main]
/// ).unwrap();
///
/// // Compute layout
/// engine.compute_layout(container, Size::MAX_CONTENT).unwrap();
///
/// // Get resulting rects
/// let header_rect = engine.get_layout_rect(header, 0, 0).unwrap();
/// let main_rect = engine.get_layout_rect(main, 0, 0).unwrap();
/// ```
/// Flexbox layout engine wrapping TaffyTree.
pub struct FlexLayoutEngine {
    tree: TaffyTree,
}

impl std::fmt::Debug for FlexLayoutEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlexLayoutEngine")
            .field("node_count", &self.tree.total_node_count())
            .finish()
    }
}

impl FlexLayoutEngine {
    /// Create a new flex layout engine.
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
        }
    }

    /// Create a leaf node with the given style.
    ///
    /// Returns a `NodeId` that can be used to reference this node.
    pub fn new_leaf(&mut self, style: taffy::Style) -> anyhow::Result<NodeId> {
        Ok(self.tree.new_leaf(style)?)
    }

    /// Create a container node with children.
    ///
    /// The children will be laid out according to the container's flex properties.
    pub fn new_with_children(
        &mut self,
        style: taffy::Style,
        children: &[NodeId],
    ) -> anyhow::Result<NodeId> {
        Ok(self.tree.new_with_children(style, children)?)
    }

    /// Add a child to an existing node.
    pub fn add_child(&mut self, parent: NodeId, child: NodeId) -> anyhow::Result<()> {
        Ok(self.tree.add_child(parent, child)?)
    }

    /// Remove a child from a parent node.
    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) -> anyhow::Result<NodeId> {
        Ok(self.tree.remove_child(parent, child)?)
    }

    /// Remove all children from a node.
    pub fn clear_children(&mut self, parent: NodeId) -> anyhow::Result<()> {
        // Get children and remove them one by one
        let children: Vec<NodeId> = self.tree.children(parent)?.into_iter().collect();
        for child in children {
            self.tree.remove_child(parent, child)?;
        }
        Ok(())
    }

    /// Get the children of a node.
    pub fn children(&self, parent: NodeId) -> anyhow::Result<Vec<NodeId>> {
        Ok(self.tree.children(parent)?.into_iter().collect())
    }

    /// Compute layout for the given node within the available space.
    ///
    /// This must be called before `get_layout_rect` to calculate positions.
    pub fn compute_layout(
        &mut self,
        node: NodeId,
        available_space: Size<AvailableSpace>,
    ) -> anyhow::Result<()> {
        Ok(self.tree.compute_layout(node, available_space)?)
    }

    /// Get the computed layout for a node as a ratatui Rect.
    ///
    /// The parent_x and parent_y are added to the node's position to get
    /// absolute coordinates.
    pub fn get_layout_rect(
        &self,
        node: NodeId,
        parent_x: u16,
        parent_y: u16,
    ) -> anyhow::Result<Rect> {
        let layout = self.tree.layout(node)?;
        Ok(Rect::new(
            parent_x + layout.location.x as u16,
            parent_y + layout.location.y as u16,
            layout.size.width as u16,
            layout.size.height as u16,
        ))
    }

    /// Convenience method: compute layout and return all child rects.
    ///
    /// This is useful when you have a container and want to get the
    /// layout rectangles for all its children.
    pub fn compute_and_split(&mut self, parent: NodeId, area: Rect) -> anyhow::Result<Vec<Rect>> {
        let available_space = Size {
            width: AvailableSpace::Definite(area.width as f32),
            height: AvailableSpace::Definite(area.height as f32),
        };

        self.compute_layout(parent, available_space)?;

        let mut rects = Vec::new();
        for child_id in self.tree.children(parent)? {
            rects.push(self.get_layout_rect(child_id, area.x, area.y)?);
        }

        Ok(rects)
    }

    /// Get the underlying TaffyTree for advanced usage.
    pub fn tree(&self) -> &TaffyTree {
        &self.tree
    }

    /// Get a mutable reference to the underlying TaffyTree.
    pub fn tree_mut(&mut self) -> &mut TaffyTree {
        &mut self.tree
    }

    /// Set the style of a node.
    pub fn set_style(&mut self, node: NodeId, style: taffy::Style) -> anyhow::Result<()> {
        Ok(self.tree.set_style(node, style)?)
    }

    /// Get the style of a node.
    pub fn style(&self, node: NodeId) -> anyhow::Result<&taffy::Style> {
        Ok(self.tree.style(node)?)
    }
}

impl Default for FlexLayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for common layout patterns.
///
/// This provides a more ergonomic API for creating common flex layouts.
///
/// # Example
///
/// ```ignore
/// use splunk_tui::ui::layout::flex::FlexLayoutBuilder;
///
/// let (engine, root) = FlexLayoutBuilder::new()
///     .column()
///     .add_fixed_child(100.0, 3.0).unwrap().0
///     .add_flex_child(1.0).unwrap().0
///     .add_fixed_child(100.0, 3.0).unwrap().0
///     .build()
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct FlexLayoutBuilder {
    engine: FlexLayoutEngine,
    root: Option<NodeId>,
}

impl FlexLayoutBuilder {
    /// Create a new flex layout builder.
    pub fn new() -> Self {
        Self {
            engine: FlexLayoutEngine::new(),
            root: None,
        }
    }

    /// Create a column layout (vertical flex).
    pub fn column(mut self) -> Self {
        let root = self
            .engine
            .new_leaf(taffy::Style {
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: percent(100.0),
                    height: percent(100.0),
                },
                ..Default::default()
            })
            .unwrap();
        self.root = Some(root);
        self
    }

    /// Create a row layout (horizontal flex).
    pub fn row(mut self) -> Self {
        let root = self
            .engine
            .new_leaf(taffy::Style {
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: percent(100.0),
                    height: percent(100.0),
                },
                ..Default::default()
            })
            .unwrap();
        self.root = Some(root);
        self
    }

    /// Add a child with flex grow.
    ///
    /// Returns the builder and the child node ID for further configuration.
    pub fn add_flex_child(mut self, grow: f32) -> anyhow::Result<(Self, NodeId)> {
        let child = self.engine.new_leaf(taffy::Style {
            flex_grow: grow,
            ..Default::default()
        })?;

        if let Some(root) = self.root {
            self.engine.add_child(root, child)?;
        }

        Ok((self, child))
    }

    /// Add a child with fixed size in pixels.
    ///
    /// Returns the builder and the child node ID for further configuration.
    pub fn add_fixed_child(mut self, width: f32, height: f32) -> anyhow::Result<(Self, NodeId)> {
        let child = self.engine.new_leaf(taffy::Style {
            size: Size {
                width: length(width),
                height: length(height),
            },
            ..Default::default()
        })?;

        if let Some(root) = self.root {
            self.engine.add_child(root, child)?;
        }

        Ok((self, child))
    }

    /// Add a child with percentage-based size.
    ///
    /// Returns the builder and the child node ID for further configuration.
    pub fn add_percent_child(
        mut self,
        width_percent: f32,
        height_percent: f32,
    ) -> anyhow::Result<(Self, NodeId)> {
        let child = self.engine.new_leaf(taffy::Style {
            size: Size {
                width: percent(width_percent),
                height: percent(height_percent),
            },
            ..Default::default()
        })?;

        if let Some(root) = self.root {
            self.engine.add_child(root, child)?;
        }

        Ok((self, child))
    }

    /// Add a child with custom style.
    ///
    /// Returns the builder and the child node ID for further configuration.
    pub fn add_child_with_style(mut self, style: taffy::Style) -> anyhow::Result<(Self, NodeId)> {
        let child = self.engine.new_leaf(style)?;

        if let Some(root) = self.root {
            self.engine.add_child(root, child)?;
        }

        Ok((self, child))
    }

    /// Build and return the engine and root node.
    pub fn build(self) -> anyhow::Result<(FlexLayoutEngine, NodeId)> {
        match self.root {
            Some(root) => Ok((self.engine, root)),
            None => anyhow::bail!("No root node created. Call column() or row() first."),
        }
    }

    /// Get a reference to the underlying engine.
    pub fn engine(&self) -> &FlexLayoutEngine {
        &self.engine
    }

    /// Get a mutable reference to the underlying engine.
    pub fn engine_mut(&mut self) -> &mut FlexLayoutEngine {
        &mut self.engine
    }
}

impl Default for FlexLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating common layout patterns.
pub mod helpers {
    use super::*;

    /// Create a standard header/main/footer layout.
    ///
    /// Header and footer have fixed heights, main grows to fill remaining space.
    /// Returns the engine, container node, and the three child nodes.
    pub fn header_main_footer(
        header_height: f32,
        footer_height: f32,
    ) -> anyhow::Result<(FlexLayoutEngine, NodeId, NodeId, NodeId, NodeId)> {
        let mut engine = FlexLayoutEngine::new();

        let header = engine.new_leaf(taffy::Style {
            size: Size {
                width: percent(100.0),
                height: length(header_height),
            },
            ..Default::default()
        })?;

        let main = engine.new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: percent(100.0),
                height: auto(),
            },
            ..Default::default()
        })?;

        let footer = engine.new_leaf(taffy::Style {
            size: Size {
                width: percent(100.0),
                height: length(footer_height),
            },
            ..Default::default()
        })?;

        // Container fills available space
        let container = engine.new_with_children(
            taffy::Style {
                flex_direction: FlexDirection::Column,
                size: Size {
                    width: percent(100.0),
                    height: percent(100.0),
                },
                ..Default::default()
            },
            &[header, main, footer],
        )?;

        Ok((engine, container, header, main, footer))
    }

    /// Create a sidebar/main content layout.
    ///
    /// Sidebar has fixed width, main grows to fill remaining space.
    pub fn sidebar_main(
        sidebar_width: f32,
    ) -> anyhow::Result<(FlexLayoutEngine, NodeId, NodeId, NodeId)> {
        let mut engine = FlexLayoutEngine::new();

        let sidebar = engine.new_leaf(taffy::Style {
            size: Size {
                width: length(sidebar_width),
                height: percent(100.0),
            },
            ..Default::default()
        })?;

        let main = engine.new_leaf(taffy::Style {
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: percent(100.0),
            },
            ..Default::default()
        })?;

        // Container fills available space
        let container = engine.new_with_children(
            taffy::Style {
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: percent(100.0),
                    height: percent(100.0),
                },
                ..Default::default()
            },
            &[sidebar, main],
        )?;

        Ok((engine, container, sidebar, main))
    }

    /// Create an equal column layout.
    pub fn equal_columns(count: usize) -> anyhow::Result<(FlexLayoutEngine, NodeId, Vec<NodeId>)> {
        let mut engine = FlexLayoutEngine::new();
        let mut columns = Vec::with_capacity(count);

        for _ in 0..count {
            let col = engine.new_leaf(taffy::Style {
                flex_grow: 1.0,
                size: Size {
                    width: auto(),
                    height: percent(100.0),
                },
                ..Default::default()
            })?;
            columns.push(col);
        }

        // Container fills available space
        let container = engine.new_with_children(
            taffy::Style {
                flex_direction: FlexDirection::Row,
                size: Size {
                    width: percent(100.0),
                    height: percent(100.0),
                },
                ..Default::default()
            },
            &columns,
        )?;

        Ok((engine, container, columns))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_layout_engine_creation() {
        let engine = FlexLayoutEngine::new();
        // Just verify it creates without panic
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
                    height: length(3.0),
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
                    height: length(3.0),
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
                        width: length(100.0),
                        height: length(24.0),
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

        // Verify footer rect
        let footer_rect = engine.get_layout_rect(footer, 0, 0).unwrap();
        assert_eq!(footer_rect.height, 3);

        // Verify main takes remaining space
        let main_rect = engine.get_layout_rect(main, 0, 0).unwrap();
        assert_eq!(main_rect.height, 18); // 24 - 3 - 3
    }

    #[test]
    fn test_compute_and_split() {
        let mut engine = FlexLayoutEngine::new();

        let header = engine
            .new_leaf(taffy::Style {
                size: Size {
                    width: percent(100.0),
                    height: length(3.0),
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
                        width: length(80.0),
                        height: length(24.0),
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
    }

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
    fn test_add_child_after_build() {
        let mut engine = FlexLayoutEngine::new();

        let parent = engine
            .new_leaf(taffy::Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            })
            .unwrap();

        let child = engine
            .new_leaf(taffy::Style {
                size: Size {
                    width: length(10.0),
                    height: length(10.0),
                },
                ..Default::default()
            })
            .unwrap();

        engine.add_child(parent, child).unwrap();

        let children: Vec<_> = engine.children(parent).unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0], child);
    }
}
