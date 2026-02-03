use glam::Vec2;

/// Identifies an edge position of a rectangle.
///
/// If `N` is `true`, Y is up.
/// If `N` is `false`, Y is down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgePosition<const N: bool = true> {
    Top,
    Right,
    Bottom,
    Left,
    None,
}

impl<const N: bool> EdgePosition<N> {
    /// Casts an edge position to a vector.
    #[inline]
    pub const fn as_vec2(&self) -> Vec2 {
        match self {
            Self::Top => {
                if N {
                    Vec2::Y
                } else {
                    Vec2::NEG_Y
                }
            }
            Self::Right => Vec2::X,
            Self::Bottom => {
                if N {
                    Vec2::NEG_Y
                } else {
                    Vec2::Y
                }
            }
            Self::Left => Vec2::NEG_X,
            Self::None => Vec2::ZERO,
        }
    }
}

/// Identifies an edge type of a connector.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum EdgeType {
    #[default]
    Curve,
    SmoothStep,
    Straight,
    StraightStep,
}
