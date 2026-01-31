use bevy_math::Vec2;

#[inline]
pub const fn map_fn<T, F>(f: F, s: f32) -> impl Fn(T) -> T + Copy
where
    F: Fn(T, f32) -> T + Copy,
{
    move |v| f(v, s)
}

/// Converts a taffy unit to a bevy unit.
pub trait Convert {
    type Output;

    #[must_use]
    fn convert(self) -> Self::Output;
}

/// Scales a taffy unit with a given scale factor.
pub trait Scale<T = Self> {
    #[must_use]
    fn scale(self, factor: f32) -> T;
}

impl<T: Scale> Scale for Vec<T> {
    fn scale(self, factor: f32) -> Self {
        let f = map_fn::<T, _>(|t, s| t.scale(s), factor);
        self.into_iter().map(f).collect()
    }
}

/// Resolves a taffy unit with a given basic value.
pub trait Resolve {
    type Output;

    #[must_use]
    fn resolve(self, base: f32) -> Self::Output;
}

impl<T: Resolve, const N: usize> Resolve for [T; N] {
    type Output = [T::Output; N];

    fn resolve(self, base: f32) -> Self::Output {
        self.map(|v| v.resolve(base))
    }
}

pub const VEC2_FLIP_X: Vec2 = Vec2::new(-1.0, 1.0);

pub const VEC2_FLIP_Y: Vec2 = Vec2::new(1.0, -1.0);

/// Utilities for working with Taffy geometry.
pub mod taffy {
    use bevy_math::Vec2;

    use super::{Convert, Resolve, Scale};

    impl Convert for taffy::Point<f32> {
        type Output = Vec2;

        fn convert(self) -> Self::Output {
            Vec2::new(self.x, self.y)
        }
    }

    impl Convert for taffy::Size<f32> {
        type Output = Vec2;

        fn convert(self) -> Self::Output {
            Vec2::new(self.width, self.height)
        }
    }

    impl Convert for taffy::Rect<f32> {
        type Output = [f32; 4];

        /// Returns the values as an array in a counter-clockwise order.
        fn convert(self) -> Self::Output {
            [self.left, self.bottom, self.right, self.top]
        }
    }

    impl Scale for taffy::Point<f32> {
        fn scale(self, factor: f32) -> Self {
            self.map(|x| x * factor)
        }
    }

    impl<T: Scale> Scale for taffy::Size<T> {
        fn scale(self, factor: f32) -> Self {
            self.map(|x| x.scale(factor))
        }
    }

    impl Scale for taffy::Size<f32> {
        fn scale(self, factor: f32) -> Self {
            self.map(|x| x * factor)
        }
    }

    impl Scale for Option<f32> {
        fn scale(self, factor: f32) -> Self {
            self.map(|v| v * factor)
        }
    }

    impl<T: Scale> Scale for taffy::Rect<T> {
        fn scale(self, factor: f32) -> Self {
            self.map(|x| x.scale(factor))
        }
    }

    impl Scale for taffy::Rect<f32> {
        fn scale(self, factor: f32) -> Self {
            self.map(|x| x * factor)
        }
    }

    impl Scale for taffy::CompactLength {
        fn scale(self, factor: f32) -> Self {
            match self.tag() {
                Self::LENGTH_TAG => Self::length(self.value() * factor),
                Self::FIT_CONTENT_PX_TAG => Self::fit_content_px(self.value() * factor),
                _ => self,
            }
        }
    }

    impl Scale for taffy::Dimension {
        fn scale(self, factor: f32) -> Self {
            let raw = self.into_raw();
            if raw.tag() == taffy::CompactLength::LENGTH_TAG {
                Self::length(raw.value() * factor)
            } else {
                self
            }
        }
    }

    impl Scale for taffy::LengthPercentage {
        fn scale(self, factor: f32) -> Self {
            let raw = self.into_raw();
            if raw.tag() == taffy::CompactLength::LENGTH_TAG {
                Self::length(raw.value() * factor)
            } else {
                self
            }
        }
    }

    impl Scale for taffy::LengthPercentageAuto {
        fn scale(self, factor: f32) -> Self {
            let raw = self.into_raw();
            if raw.tag() == taffy::CompactLength::LENGTH_TAG {
                Self::length(raw.value() * factor)
            } else {
                self
            }
        }
    }

    impl Scale for taffy::AvailableSpace {
        fn scale(self, factor: f32) -> Self {
            match self {
                Self::Definite(x) => Self::Definite(x * factor),
                _ => self,
            }
        }
    }

    impl Scale for taffy::MinTrackSizingFunction {
        fn scale(self, factor: f32) -> Self {
            let raw = self.into_raw();
            match raw.tag() {
                taffy::CompactLength::LENGTH_TAG => Self::length(raw.value() * factor),
                _ => self,
            }
        }
    }

    impl Scale for taffy::MaxTrackSizingFunction {
        fn scale(self, factor: f32) -> Self {
            let raw = self.into_raw();
            match raw.tag() {
                taffy::CompactLength::LENGTH_TAG => Self::length(raw.value() * factor),
                taffy::CompactLength::FIT_CONTENT_PX_TAG => {
                    Self::fit_content_px(raw.value() * factor)
                }
                _ => self,
            }
        }
    }

    impl Scale for taffy::MinMax<taffy::MinTrackSizingFunction, taffy::MaxTrackSizingFunction> {
        fn scale(self, factor: f32) -> Self {
            Self {
                min: self.min.scale(factor),
                max: self.max.scale(factor),
            }
        }
    }

    impl<S: taffy::CheapCloneStr> Scale for taffy::GridTemplateRepetition<S> {
        fn scale(self, factor: f32) -> Self {
            Self {
                tracks: self.tracks.scale(factor),
                ..self
            }
        }
    }

    impl<S: taffy::CheapCloneStr> Scale for taffy::GridTemplateComponent<S> {
        fn scale(self, factor: f32) -> Self {
            match self {
                Self::Single(inner) => Self::Single(inner.scale(factor)),
                Self::Repeat(inner) => Self::Repeat(inner.scale(factor)),
            }
        }
    }

    impl Scale for taffy::Layout {
        fn scale(self, factor: f32) -> Self {
            Self {
                order: self.order,
                location: self.location.scale(factor),
                size: self.size.scale(factor),
                content_size: self.content_size.scale(factor),
                scrollbar_size: self.scrollbar_size.scale(factor),
                border: self.border.scale(factor),
                padding: self.padding.scale(factor),
                margin: self.margin.scale(factor),
            }
        }
    }

    impl Resolve for taffy::LengthPercentage {
        type Output = f32;

        fn resolve(self, base: f32) -> Self::Output {
            let raw = self.into_raw();
            match raw.tag() {
                taffy::CompactLength::LENGTH_TAG => raw.value(),
                taffy::CompactLength::PERCENT_TAG => raw.value() * base,
                _ => 0.0,
            }
        }
    }
}
