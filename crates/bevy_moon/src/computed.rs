use std::f32;

use bevy_color::Alpha;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Affine3A, Rect, UVec2, Vec2};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_sprite::BorderRect;

use crate::{
    properties::overflow::{OverflowClipMargin, OverflowVisualBox},
    style::{Corners, Outline},
};

/// Provides the computed size and layout properties of the node.
#[derive(Component, Debug, Copy, Clone, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct ComputedNode {
    /// The order of the node in the UI layout.
    /// Nodes with a higher stack index are drawn on top of and receive interactions before nodes with lower stack indices.
    ///
    /// Automatically calculated in [`UiSystems::Stack`](`super::UiSystems::Stack`).
    pub stack_index: usize,

    /// The relative ordering of the node
    ///
    /// Nodes with a higher order should be rendered on top of those with a lower order.
    /// This is effectively a topological sort of each tree.
    pub order: u32,

    /// The top-left corner of the node.
    pub location: Vec2,

    /// The width and height of the node.
    pub size: Vec2,

    /// The width and height of the content inside the node. This may be larger than the size of the node in the case of
    /// overflowing content and is useful for computing a "scroll width/height" for scrollable nodes
    pub content_size: Vec2,

    /// The size of the scrollbars in each dimension. If there is no scrollbar then the size will be zero.
    pub scrollbar_size: Vec2,

    /// Theses values as an array in a counter-clockwise order: [left, bottom, right, top].
    /// The size of the borders of the node
    pub border: [f32; 4],
    /// The size of the padding of the node
    pub padding: [f32; 4],
    /// The size of the margin of the node
    pub margin: [f32; 4],

    /// Theses values as an array in a counter-clockwise order: [bottom-left, bottom-right, top-right, top-left].
    /// The size of the corner radii of the node
    pub corner_radii: [f32; 4],

    // [width, offset]
    pub outline: [f32; 2],

    /// The size of the content box of the node.
    pub content_box_size: Vec2,

    /// This is the affine of the node relative to its parent.
    /// Stores it for inversion.
    pub affine: Affine3A,
}

impl Default for ComputedNode {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ComputedNode {
    pub const DEFAULT: Self = Self {
        order: 0,
        location: Vec2::ZERO,
        size: Vec2::ZERO,
        content_size: Vec2::ZERO,
        scrollbar_size: Vec2::ZERO,
        border: [0.0; 4],
        padding: [0.0; 4],
        margin: [0.0; 4],

        stack_index: 0,
        corner_radii: [0.0; 4],
        outline: [0.0; 2],
        content_box_size: Vec2::ZERO,

        affine: Affine3A::IDENTITY,
    };

    pub fn is_empty(&self) -> bool {
        self.size.cmpeq(Vec2::ZERO).any()
    }

    pub fn set_corner_radii(&mut self, corner_radii: Corners<f32>) {
        use crate::geometry::Resolve;

        let min_length = self.size.x.min(self.size.y);
        self.corner_radii = corner_radii.resolve(min_length).into_array();
    }

    pub fn set_outline(&mut self, outline: Outline) {
        use crate::geometry::Resolve;

        let Outline {
            color,
            width,
            offset,
        } = outline;

        if color.is_fully_transparent() {
            return;
        }

        self.outline = [width, offset].resolve(self.size.x);
    }

    pub fn outline_size(&self) -> Vec2 {
        let distance = self.outline[0] + self.outline[1];
        self.size + distance * 2.0
    }

    pub fn outline_border_width(&self) -> [f32; 4] {
        [self.outline[0]; 4]
    }

    pub fn outline_corner_radii(&self) -> [f32; 4] {
        let distance = self.outline[0] + self.outline[1];
        self.corner_radii.map(|v| v + distance)
    }

    pub fn border_inset(&self) -> BorderRect {
        let [left, bottom, right, top] = self.border;
        [left, right, bottom, top].into()
    }

    pub fn padding_inset(&self) -> BorderRect {
        let [left, bottom, right, top] = self.padding;
        [left, right, bottom, top].into()
    }

    pub fn content_box_inset(&self) -> BorderRect {
        self.border_inset() + self.padding_inset()
    }

    pub fn clip_rect(
        &self,
        center: Vec2,
        (x_visible, y_visible): (bool, bool),
        overflow_clip_margin: OverflowClipMargin,
    ) -> Option<Rect> {
        if x_visible && y_visible {
            return None;
        }

        let OverflowClipMargin { visual_box, margin } = overflow_clip_margin;

        let mut clip_rect = Rect::from_center_size(center, self.size);

        let clip_inset = match visual_box {
            OverflowVisualBox::BorderBox => BorderRect::ZERO,
            OverflowVisualBox::PaddingBox => self.border_inset(),
            OverflowVisualBox::ContentBox => self.content_box_inset(),
        };

        // bottom-left
        clip_rect.min += clip_inset.min_inset;
        // top-right
        clip_rect.max -= clip_inset.max_inset;

        clip_rect = clip_rect.inflate(margin.max(0.));

        if x_visible {
            clip_rect.min.x = f32::NEG_INFINITY;
            clip_rect.max.x = f32::INFINITY;
        }
        if y_visible {
            clip_rect.min.y = f32::NEG_INFINITY;
            clip_rect.max.y = f32::INFINITY;
        }

        Some(clip_rect)
    }

    pub fn set_layout(&mut self, layout: taffy::Layout) {
        use crate::geometry::Convert;

        let content_box_size = layout.content_box_size();

        let taffy::Layout {
            location,
            size,
            content_size,
            scrollbar_size,
            border,
            padding,
            margin,
            order,
        } = layout;

        self.order = order;
        self.location = location.convert();
        self.size = size.convert();
        self.content_size = content_size.convert();
        self.scrollbar_size = scrollbar_size.convert();
        self.border = border.convert();
        self.padding = padding.convert();
        self.margin = margin.convert();
        self.content_box_size = content_box_size.convert();
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ComputedTargetInfo {
    pub scale_factor: f32,
    pub zoom_factor: f32,
    pub physical_size: UVec2,
}
