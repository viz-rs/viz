use glam::Vec2;

use crate::edge::EdgePosition;

pub struct StepsBuilder {
    pub points: Vec<Vec2>,
    pub offset: f32,
}

impl StepsBuilder {
    pub fn new(source: (Vec2, EdgePosition), target: (Vec2, EdgePosition), offset: f32) -> Self {
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

        let mut points = Vec::with_capacity(3);

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

        Self { points, offset }
    }
}

/// Calculates the visible area of a rectangle defined by its minimum and maximum coordinates.
#[inline]
const fn visible_area(min: Vec2, max: Vec2) -> f32 {
    let x = (max.x - min.x).max(0.0);
    let y = (max.y - min.y).max(0.0);
    x * y
}

/// Selects a vector based on a flag.
///
/// If the flag is 1.0, returns the maximum of the base and max vectors.
/// If the flag is -1.0, returns the minimum of the base and min vectors.
/// Otherwise, returns the base vector.
#[inline]
const fn select(flags: Vec2, base: Vec2, min: Vec2, max: Vec2) -> Vec2 {
    Vec2 {
        x: select_single(flags.x, base.x, min.x, max.x),
        y: select_single(flags.y, base.y, min.y, max.y),
    }
}

/// Selects a single value based on a flag.
///
/// If the flag is 1.0, returns the maximum of the base and max values.
/// If the flag is -1.0, returns the minimum of the base and min values.
/// Otherwise, returns the base value.
#[inline]
const fn select_single(flag: f32, base: f32, min: f32, max: f32) -> f32 {
    if flag == 1.0 {
        base.max(max)
    } else if flag == -1.0 {
        base.min(min)
    } else {
        base
    }
}
