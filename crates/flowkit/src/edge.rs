use glam::Vec2;

use crate::{CURVATURE, OFFSET};

/// Identifies an edge type of a connector.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum EdgeType {
    #[default]
    Curve,
    SmoothStep,
    Straight,
    StraightStep,
}

/// Identifies an edge anchor of a rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeAnchor {
    Top,
    Right,
    Bottom,
    Left,
    None,
}

impl EdgeAnchor {
    /// Casts an edge anchor to a vector.
    #[inline]
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

/// Identifies an edge point of a connector.
///
/// Includes the position and the anchor.
pub type EdgePoint = (Vec2, EdgeAnchor);

/// Generates an edge path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgePath {
    pub source: EdgePoint,
    pub target: EdgePoint,
    pub edge_type: EdgeType,
    pub curvature: f32,
    pub offset: f32,
}

impl Default for EdgePath {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl EdgePath {
    pub const DEFAULT: Self = Self {
        source: (Vec2::ZERO, EdgeAnchor::Right),
        target: (Vec2::ZERO, EdgeAnchor::Left),
        edge_type: EdgeType::Straight,
        curvature: CURVATURE,
        offset: OFFSET,
    };
}
