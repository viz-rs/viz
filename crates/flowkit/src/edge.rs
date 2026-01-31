use glam::Vec2;

/// Identifies an edge position of a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgePosition {
    Top,
    Right,
    Bottom,
    Left,
    None,
}

impl EdgePosition {
    /// Casts an edge position to a vector.
    pub const fn as_vec2(&self) -> Vec2 {
        match self {
            Self::Top => Vec2::Y,
            Self::Right => Vec2::X,
            Self::Bottom => Vec2::NEG_Y,
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
