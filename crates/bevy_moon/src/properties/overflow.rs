use bevy_reflect::{Reflect, prelude::ReflectDefault};

/// Used to determine the bounds of the visible area when a UI node is clipped.
///
/// Spec: <https://drafts.csswg.org/css-box-4/#typedef-visual-box>
#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Reflect)]
#[reflect(Default, PartialEq, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum OverflowVisualBox {
    /// Clip any content that overflows outside the border box
    BorderBox,
    /// Clip any content that overflows outside the content box
    ContentBox,
    /// Clip any content that overflows outside the padding box
    #[default]
    PaddingBox,
}

/// The bounds of the visible area when a UI node is clipped.
///
/// Spec: <https://developer.mozilla.org/docs/Web/CSS/overflow-clip-margin>
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(Default, PartialEq, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct OverflowClipMargin {
    /// Visible unclipped area
    pub visual_box: OverflowVisualBox,
    /// Width of the margin on each edge of the visual box in logical pixels.
    /// The width of the margin will be zero if a negative value is set.
    pub margin: f32,
}

impl OverflowClipMargin {
    pub const DEFAULT: Self = Self {
        visual_box: OverflowVisualBox::PaddingBox,
        margin: 0.,
    };

    /// Clip any content that overflows outside the content box
    pub const fn content_box() -> Self {
        Self {
            visual_box: OverflowVisualBox::ContentBox,
            ..Self::DEFAULT
        }
    }

    /// Clip any content that overflows outside the padding box
    pub const fn padding_box() -> Self {
        Self {
            visual_box: OverflowVisualBox::PaddingBox,
            ..Self::DEFAULT
        }
    }

    /// Clip any content that overflows outside the border box
    pub const fn border_box() -> Self {
        Self {
            visual_box: OverflowVisualBox::BorderBox,
            ..Self::DEFAULT
        }
    }

    /// Add a margin on each edge of the visual box in logical pixels.
    /// The width of the margin will be zero if a negative value is set.
    pub const fn with_margin(mut self, margin: f32) -> Self {
        self.margin = margin;
        self
    }
}

impl Default for OverflowClipMargin {
    fn default() -> Self {
        Self::DEFAULT
    }
}
