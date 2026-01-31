//! Follows RectangleMeshBuilder
//!
//! https://docs.rs/bevy_mesh/0.18.0/src/bevy_mesh/primitives/dim2.rs.html#1055-1077

use bevy_math::Vec2;

pub(crate) const VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(0.5, 0.5),   // top-right
    Vec2::new(-0.5, 0.5),  // top-left
    Vec2::new(-0.5, -0.5), // bottom-left
    Vec2::new(0.5, -0.5),  // bottom-right
];

// counter-clockwise order
pub(crate) const INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

pub(crate) const UVS: [Vec2; 4] = [Vec2::X, Vec2::ZERO, Vec2::Y, Vec2::ONE];
