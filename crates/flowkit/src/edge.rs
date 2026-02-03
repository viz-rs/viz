use glam::Vec2;

use crate::{CURVATURE, OFFSET};

/// Identifies an edge anchor of a rectangle.
///
/// If `Y` is `true`, Y-axis is up.
/// If `Y` is `false`, Y-axis is down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeAnchor<const Y: bool = true> {
    Top,
    Right,
    Bottom,
    Left,
    None,
}

impl<const Y: bool> EdgeAnchor<Y> {
    /// Casts an edge position to a vector.
    #[inline]
    pub const fn as_vec2(&self) -> Vec2 {
        match self {
            Self::Top => Vec2::new(0.0, if Y { 1.0 } else { -1.0 }),
            Self::Right => Vec2::X,
            Self::Bottom => Vec2::new(0.0, if Y { -1.0 } else { 1.0 }),
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

/// Identifies an edge point of a connector.
///
/// Includes the position and the anchor.
///
/// If `Y` is `true`, Y-axis is up.
/// If `Y` is `false`, Y-axis is down.
pub type EdgePoint<const Y: bool = true> = (Vec2, EdgeAnchor<Y>);

/// Draws an edge path.
///
/// If `Y` is `true`, Y-axis is up.
/// If `Y` is `false`, Y-axis is down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgePath<const Y: bool = true> {
    pub source: EdgePoint<Y>,
    pub target: EdgePoint<Y>,
    pub edge_type: EdgeType,
    pub curvature: f32,
    pub offset: f32,
}

impl<const Y: bool> Default for EdgePath<Y> {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl<const Y: bool> EdgePath<Y> {
    pub const DEFAULT: Self = Self {
        source: (Vec2::ZERO, EdgeAnchor::<Y>::Right),
        target: (Vec2::ZERO, EdgeAnchor::<Y>::Left),
        edge_type: EdgeType::Straight,
        curvature: CURVATURE,
        offset: OFFSET,
    };
}
