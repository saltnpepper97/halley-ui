//! Small measured-content layouts for incremental migrations of existing renderers.
use crate::{Insets, Rect as UiRect, Size as UiSize};
use taffy::{TaffyTree, prelude::*};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaddedLabelLayout {
    pub card: UiSize,
    pub label: UiRect,
}
impl PaddedLabelLayout {
    /// Uses the caller's existing cached glyph measurements; no extra rasterization.
    pub fn new(measured: UiSize, padding: Insets) -> Result<Self, taffy::TaffyError> {
        let mut tree = TaffyTree::<()>::new();
        tree.disable_rounding();
        let label = tree.new_leaf(Style {
            size: Size {
                width: length(measured.width),
                height: length(measured.height),
            },
            ..Default::default()
        })?;
        let root = tree.new_with_children(
            Style {
                padding: Rect {
                    left: length(padding.left),
                    right: length(padding.right),
                    top: length(padding.top),
                    bottom: length(padding.bottom),
                },
                ..Default::default()
            },
            &[label],
        )?;
        tree.compute_layout(root, Size::MAX_CONTENT)?;
        let card = tree.layout(root)?;
        let text = tree.layout(label)?;
        Ok(Self {
            card: UiSize::new(card.size.width, card.size.height),
            label: UiRect::new(
                text.location.x,
                text.location.y,
                text.size.width,
                text.size.height,
            ),
        })
    }
}
