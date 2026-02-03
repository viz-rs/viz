use glam::Vec2;
use lyon_path::{BuilderImpl, builder::WithSvg};
use smallvec::SmallVec;

use crate::{
    corner::{Corner, CornerPathParams},
    curve::calculate_control_point,
    edge::{EdgePath, EdgePoint, EdgeType},
    utils::{Convert, select, visible_area},
    winding_order::WindingOrder,
};

/// A path builder.
///
/// If `Y` is `true`, Y-axis is up.
/// If `Y` is `false`, Y-axis is down.
#[derive(Debug, Clone)]
pub struct PathBuilder<const Y: bool = true> {
    pub points: SmallVec<[Vec2; 2]>,
    pub edge_type: EdgeType,
    pub curvature: f32,
    pub offset: f32,
}

impl<const Y: bool> PathBuilder<Y> {
    #[inline]
    pub fn new(
        source: EdgePoint<Y>,
        target: EdgePoint<Y>,
        edge_type: EdgeType,
        curvature: f32,
        offset: f32,
    ) -> Self {
        let mut points = SmallVec::new_const();

        match edge_type {
            EdgeType::Straight => {
                points.extend([source.0, target.0]);
            }
            EdgeType::Curve => {
                points.extend(Self::calculate_control_points(
                    source, target, curvature, offset,
                ));
            }
            EdgeType::StraightStep | EdgeType::SmoothStep => {
                points.extend(Self::calculate_steps(source, target, offset));
            }
        }

        Self {
            points,
            edge_type,
            curvature,
            offset,
        }
    }

    #[inline]
    pub fn calculate_control_points(
        source: EdgePoint<Y>,
        target: EdgePoint<Y>,
        curvature: f32,
        offset: f32,
    ) -> [Vec2; 4] {
        let (source_pos, source_edge) = source;
        let (target_pos, target_edge) = target;

        let source_control_point =
            calculate_control_point::<Y>(source_pos, target_pos, source_edge, curvature, offset);
        let target_control_point =
            calculate_control_point::<Y>(target_pos, source_pos, target_edge, curvature, offset);

        [
            source_pos,
            source_control_point,
            target_control_point,
            target_pos,
        ]
    }

    #[inline]
    pub fn calculate_steps(
        source: EdgePoint<Y>,
        target: EdgePoint<Y>,
        offset: f32,
    ) -> SmallVec<[Vec2; 3]> {
        let (source_pos, source_edge) = source;
        let (target_pos, target_edge) = target;

        let (rect_min, rect_max) = (source_pos.min(target_pos), source_pos.max(target_pos));
        let area = visible_area(rect_min, rect_max);

        let (source_edge_vec2, target_edge_vec2) = (source_edge.as_vec2(), target_edge.as_vec2());
        let (source_offset, target_offset) = (source_edge_vec2 * offset, target_edge_vec2 * offset);

        let (source_offset_pos, target_offset_pos) =
            (source_pos + source_offset, target_pos + target_offset);

        let (new_rect_min, new_rect_max) = (
            rect_min.min(source_offset_pos).min(target_offset_pos),
            rect_max.max(source_offset_pos).max(target_offset_pos),
        );
        let new_area = visible_area(new_rect_min, new_rect_max);

        let center = new_rect_min.midpoint(new_rect_max);

        let edges = source_edge_vec2 * target_edge_vec2;
        let is_adjacent_edge = edges == Vec2::ZERO;
        let is_same_edge = !is_adjacent_edge && edges.cmpeq(Vec2::ONE).any();
        let is_same_area = area == new_area;

        let mut points = SmallVec::new_const();

        points.push(source_pos);

        if is_same_edge {
            // same edges
            // adds two corner points
            let sc = select(
                source_edge_vec2,
                source_offset_pos,
                new_rect_min,
                new_rect_max,
            );
            let tc = select(
                target_edge_vec2,
                target_offset_pos,
                new_rect_min,
                new_rect_max,
            );
            points.extend([sc, tc]);
        } else if is_adjacent_edge && is_same_area {
            // adjacent edges and same area
            // adds one corner point
            let c = select(
                source_edge_vec2,
                source_offset_pos,
                new_rect_min,
                new_rect_max,
            );
            points.push(c);
        } else {
            // source offset point
            let sc = select(
                source_edge_vec2,
                source_offset_pos,
                source_offset_pos.min(center),
                source_offset_pos.max(center),
            );
            // target offset point
            let tc = select(
                target_edge_vec2,
                target_offset_pos,
                target_offset_pos.min(center),
                target_offset_pos.max(center),
            );

            // source middle point
            let mut sm = select(source_edge_vec2, center, sc.min(center), sc.max(center));
            // target middle point
            let mut tm = select(target_edge_vec2, center, tc.min(center), tc.max(center));

            let mut temp = Vec::with_capacity(3);

            temp.push(sc);

            if is_adjacent_edge {
                // adjacent edges
                // adds a middle corner point
                // keeps value by multiplying with edge vector length
                sm *= source_edge_vec2.abs();
                tm *= target_edge_vec2.abs();

                temp.push(sm + tm);
            } else {
                // parallel edges
                // adds two middle corner points
                temp.push(sm);
                temp.push(tm);
            }

            temp.push(tc);
            temp.dedup();

            points.extend(temp);
        }

        points.push(target_pos);

        points
    }

    #[inline]
    fn smooth_with(self, builder: &mut WithSvg<BuilderImpl>) {
        let Self { points, offset, .. } = self;
        let len = points.len();

        if len < 2 {
            return;
        }

        builder.move_to(points[0].convert());

        // @todo(fundon): should be a configuration
        let smoothness = 0.6;

        for items in points.windows(3) {
            let [prev, current, next] = items[..] else {
                break;
            };

            let rect = (next - prev).abs();
            let max_radius = rect.x.min(rect.y) * 0.5;

            // 5.0 by default
            // @todo(fundon): should be a configuration
            let corner_radius = max_radius.min(offset * 0.5);

            CornerPathParams::new(corner_radius, max_radius, smoothness)
                .squircle(
                    current,
                    Corner::calculate(prev, current, next),
                    WindingOrder::calculate(prev, current, next),
                )
                .with(builder);
        }

        builder.line_to(points[len - 1].convert());
    }

    #[inline]
    pub fn with_svg(self, mut builder: &mut WithSvg<BuilderImpl>) {
        match self.edge_type {
            EdgeType::Straight => {
                let [from, to] = self.points[..] else {
                    panic!("Straight path needs tow points.");
                };
                builder.move_to(from.convert());
                builder.line_to(to.convert());
            }
            EdgeType::Curve => {
                let [from, ctrl1, ctrl2, to] = self.points[..] else {
                    panic!("Curve path needs four points.");
                };
                builder.move_to(from.convert());
                builder.cubic_bezier_to(ctrl1.convert(), ctrl2.convert(), to.convert());
            }
            EdgeType::StraightStep => {
                for point in self.points {
                    builder.line_to(point.convert());
                }
            }
            EdgeType::SmoothStep => {
                self.smooth_with(&mut builder);
            }
        }
    }
}

impl<const Y: bool> From<EdgePath<Y>> for PathBuilder<Y> {
    fn from(path: EdgePath<Y>) -> Self {
        Self::new(
            path.source,
            path.target,
            path.edge_type,
            path.curvature,
            path.offset,
        )
    }
}

impl<const Y: bool> From<PathBuilder<Y>> for WithSvg<BuilderImpl> {
    fn from(path: PathBuilder<Y>) -> Self {
        let mut builder = BuilderImpl::new().with_svg();
        path.with_svg(&mut builder);
        builder
    }
}
