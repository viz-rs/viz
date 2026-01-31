use crate::{
    action::Action,
    corner::{Corner, CornerPathParams},
    winding_order::WindingOrder,
};

use super::steps::StepsBuilder;

impl StepsBuilder {
    pub fn smooth(self) -> Vec<Action> {
        let Self { points, offset } = self;
        let len = points.len();
        let mut actions = Vec::with_capacity(2);

        if len < 2 {
            return actions;
        }

        let start_idx = 0;
        let end_idx = len - 1;

        if len == 2 {
            actions.push(Action::MoveTo(points[start_idx]));
            actions.push(Action::LineTo(points[end_idx]));
        }

        // @todo(fundon): should be a configuration
        let smoothness = 0.6;

        for (idx, &current) in points.iter().enumerate() {
            if idx == start_idx {
                actions.push(Action::MoveTo(current));
                continue;
            }

            if idx == end_idx {
                actions.push(Action::LineTo(current));
                continue;
            }

            let prev = points[idx - 1];
            let next = points[idx + 1];

            let rect = (next - prev).abs();
            let max_radius = rect.x.min(rect.y) * 0.5;

            // 5.0 by default
            // @todo(fundon): should be a configuration
            let corner_radius = max_radius.min(offset * 0.5);

            let corner = Corner::calculate(prev, current, next);
            let orientation = WindingOrder::calculate(prev, current, next);

            let corner_path_params = CornerPathParams::new(corner_radius, max_radius, smoothness);

            actions.extend(corner_path_params.squircle(current, corner, orientation));
        }

        actions
    }
}
