use glam::Vec2;

use crate::{curve::calculate_control_point, edge::EdgePosition};

pub struct BezierCurveBuilder {
    pub source_point: Vec2,
    pub source_control_point: Vec2,
    pub target_point: Vec2,
    pub target_control_point: Vec2,
}

impl BezierCurveBuilder {
    pub fn new(
        source: (Vec2, EdgePosition),
        target: (Vec2, EdgePosition),
        curvature: f32,
        offset: f32,
    ) -> Self {
        let (source_point, source_edge_pos) = source;
        let (target_point, target_edge_pos) = target;

        let source_control_point = calculate_control_point(
            source_point,
            target_point,
            source_edge_pos,
            curvature,
            offset,
        );
        let target_control_point = calculate_control_point(
            target_point,
            source_point,
            target_edge_pos,
            curvature,
            offset,
        );

        Self {
            source_point,
            source_control_point,
            target_point,
            target_control_point,
        }
    }
}
