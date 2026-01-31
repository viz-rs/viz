use std::fmt::Debug;

use bevy_color::Color;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Rect, Vec2};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use taffy::prelude::TaffyZero;

use crate::{
    geometry::{Resolve, map_fn},
    properties::overflow::OverflowClipMargin,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Clone, Default, PartialEq)]
pub struct Corners<T>
where
    T: Clone + Copy + Debug + Default + PartialEq + Reflect,
{
    /// The value associated with the bottom left corner.
    pub bottom_left: T,
    /// The value associated with the bottom right corner.
    pub bottom_right: T,
    /// The value associated with the top right corner.
    pub top_right: T,
    /// The value associated with the top left corner.
    pub top_left: T,
}

impl<T> Corners<T>
where
    T: Clone + Copy + Debug + Default + PartialEq + Reflect,
{
    #[inline]
    pub const fn all(value: T) -> Self {
        Self {
            bottom_left: value,
            bottom_right: value,
            top_right: value,
            top_left: value,
        }
    }

    #[inline]
    pub const fn bottom_left(self, value: T) -> Self {
        Self {
            bottom_left: value,
            ..self
        }
    }

    #[inline]
    pub const fn bottom_right(self, value: T) -> Self {
        Self {
            bottom_right: value,
            ..self
        }
    }

    #[inline]
    pub const fn top_right(self, value: T) -> Self {
        Self {
            top_right: value,
            ..self
        }
    }

    #[inline]
    pub const fn top_left(self, value: T) -> Self {
        Self {
            top_left: value,
            ..self
        }
    }

    /// Returns the corners as an array in a counter-clockwise order.
    #[inline]
    pub const fn into_array(self) -> [T; 4] {
        [
            self.bottom_left,
            self.bottom_right,
            self.top_right,
            self.top_left,
        ]
    }

    #[inline]
    pub fn map<R, F>(self, f: F) -> Corners<R>
    where
        R: Clone + Copy + Debug + Default + PartialEq + Reflect,
        F: Fn(T) -> R,
    {
        Corners {
            bottom_left: f(self.bottom_left),
            bottom_right: f(self.bottom_right),
            top_right: f(self.top_right),
            top_left: f(self.top_left),
        }
    }
}

impl Corners<f32> {
    pub const DEFAULT: Self = Self::all(0.0);

    const fn resolve_single(radius: f32, min_length: f32) -> f32 {
        radius.clamp(0.0, 0.5 * min_length)
    }
}

impl Resolve for Corners<f32> {
    type Output = Self;

    fn resolve(self, min_length: f32) -> Self::Output {
        let f = map_fn(Self::resolve_single, min_length);
        self.map(f)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Clone, Default, PartialEq)]
pub struct BorderColor {
    pub left: Color,
    pub bottom: Color,
    pub right: Color,
    pub top: Color,
}

impl BorderColor {
    #[inline]
    pub fn all<T>(value: T) -> Self
    where
        T: Into<Color>,
    {
        let color = value.into();
        Self {
            left: color,
            bottom: color,
            right: color,
            top: color,
        }
    }

    #[inline]
    pub fn left<T>(self, value: T) -> Self
    where
        T: Into<Color>,
    {
        Self {
            left: value.into(),
            ..self
        }
    }

    #[inline]
    pub fn bottom<T>(self, value: T) -> Self
    where
        T: Into<Color>,
    {
        Self {
            bottom: value.into(),
            ..self
        }
    }

    #[inline]
    pub fn right<T>(self, value: T) -> Self
    where
        T: Into<Color>,
    {
        Self {
            right: value.into(),
            ..self
        }
    }

    #[inline]
    pub fn top<T>(self, value: T) -> Self
    where
        T: Into<Color>,
    {
        Self {
            top: value.into(),
            ..self
        }
    }

    /// Returns the border color as an array in a counter-clockwise order.
    #[inline]
    pub const fn into_array(self) -> [Color; 4] {
        [self.left, self.bottom, self.right, self.top]
    }
}

/// The style of a border.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[repr(C)]
pub enum BorderStyle {
    /// A solid border.
    #[default]
    Solid = 0,
    /// A dashed border.
    Dashed = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Clone, Default, PartialEq)]
pub struct BoxShadow {
    pub color: Color,
    pub offset: Vec2,
    pub blur_radius: f32,
    pub spread_radius: f32,
}

impl Default for BoxShadow {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl BoxShadow {
    pub const DEFAULT: Self = Self {
        color: Color::BLACK,
        offset: Vec2::new(0.0, -10.0),
        blur_radius: 7.5,
        spread_radius: 5.0,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outline {
    pub color: Color,
    pub width: taffy::LengthPercentage,
    pub offset: taffy::LengthPercentage,
}

impl Default for Outline {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Outline {
    pub const DEFAULT: Self = Self {
        color: Color::NONE,
        width: taffy::LengthPercentage::ZERO,
        offset: taffy::LengthPercentage::ZERO,
    };
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Clone, Default)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Style {
    #[reflect(ignore, clone)]
    inner: taffy::Style,

    pub background: Option<Color>,

    pub border_color: Option<BorderColor>,

    // @TODO(fundon): https://www.shadertoy.com/view/4lKXWD
    pub border_style: BorderStyle,

    pub corner_radii: Corners<f32>,

    // @TODO(fundon): shoule be Vec<BoxShadow>
    pub box_shadow: Option<BoxShadow>,

    #[reflect(ignore, clone)]
    pub outline: Option<Outline>,

    pub clip_rect: Option<Rect>,
    pub overflow_clip_margin: OverflowClipMargin,
}

unsafe impl Sync for Style {}
unsafe impl Send for Style {}

impl Default for Style {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<taffy::Style> for Style {
    fn from(style: taffy::Style) -> Self {
        Self {
            inner: style,
            ..Self::DEFAULT
        }
    }
}

impl Style {
    pub const DEFAULT: Self = Self {
        inner: taffy::Style::DEFAULT,

        background: None,
        border_color: None,
        border_style: BorderStyle::Solid,
        corner_radii: Corners::DEFAULT,
        box_shadow: None,
        outline: None,

        clip_rect: None,
        overflow_clip_margin: OverflowClipMargin::DEFAULT,
    };

    #[inline]
    pub const fn new() -> Self {
        Self::DEFAULT
    }

    #[inline]
    pub fn background<T>(mut self, color: T) -> Self
    where
        T: Into<Color>,
    {
        self.background = Some(color.into());
        self
    }

    #[inline]
    pub const fn border_color(mut self, color: BorderColor) -> Self {
        self.border_color = Some(color);
        self
    }

    #[inline]
    pub const fn border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    #[inline]
    pub const fn corner_radii(mut self, radii: Corners<f32>) -> Self {
        self.corner_radii = radii;
        self
    }

    #[inline]
    pub const fn box_shadow(mut self, shadow: BoxShadow) -> Self {
        self.box_shadow = Some(shadow);
        self
    }

    pub const fn outline(mut self, outline: Outline) -> Self {
        self.outline = Some(outline);
        self
    }

    pub fn overflow_clip_margin(mut self, overflow_clip_margin: OverflowClipMargin) -> Self {
        self.overflow_clip_margin = overflow_clip_margin;
        self
    }

    #[inline]
    pub fn from_taffy(style: taffy::Style) -> Self {
        Self {
            inner: style,
            ..Default::default()
        }
    }

    #[inline]
    pub fn into_inner(self) -> taffy::Style {
        self.inner
    }

    #[inline]
    pub fn get_ref(&self) -> &taffy::Style {
        &self.inner
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut taffy::Style {
        &mut self.inner
    }

    #[inline]
    pub fn is_hidden(&self) -> bool {
        self.inner.display == taffy::Display::None
    }

    pub fn overflow_is_visible(&self) -> (bool, bool) {
        (
            self.inner.overflow.x == taffy::Overflow::Visible,
            self.inner.overflow.y == taffy::Overflow::Visible,
        )
    }

    #[inline]
    pub fn into_taffy(&self, scale_factor: f32) -> taffy::Style {
        use crate::geometry::Scale;

        let style = &self.inner;

        taffy::Style {
            inset: style.inset.scale(scale_factor),
            size: style.size.scale(scale_factor),
            min_size: style.min_size.scale(scale_factor),
            max_size: style.max_size.scale(scale_factor),
            margin: style.margin.scale(scale_factor),
            padding: style.padding.scale(scale_factor),
            border: style.border.scale(scale_factor),
            gap: style.gap.scale(scale_factor),
            flex_basis: style.flex_basis.scale(scale_factor),
            grid_template_rows: style.grid_template_rows.clone().scale(scale_factor),
            grid_template_columns: style.grid_template_columns.clone().scale(scale_factor),
            grid_auto_rows: style.grid_auto_rows.clone().scale(scale_factor),
            grid_auto_columns: style.grid_auto_columns.clone().scale(scale_factor),
            ..style.clone()
        }
    }
}
