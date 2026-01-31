use bevy_math::Vec2;
use bevy_prototype_lyon::path::ShapePath;
use flowkit::{
    action::Action,
    path::{bezier::BezierCurveBuilder, steps::StepsBuilder},
};

pub mod prelude {
    pub use flowkit::corner::{Corner, CornerPathParams};
    pub use flowkit::edge::{EdgePosition, EdgeType};
    pub use flowkit::path::bezier::BezierCurveBuilder;
    pub use flowkit::path::steps::StepsBuilder;
}

/// Draws a Bezier curve path from the source to the target.
pub fn draw_bezier_curve_path(builder: BezierCurveBuilder) -> ShapePath {
    let BezierCurveBuilder {
        source_point,
        source_control_point,
        target_point,
        target_control_point,
    } = builder;
    ShapePath::new().move_to(source_point).cubic_bezier_to(
        source_control_point,
        target_control_point,
        target_point,
    )
}

/// Draws a straight path from the source to the target.
pub fn draw_straight_path(source_point: Vec2, target_point: Vec2) -> ShapePath {
    ShapePath::new().move_to(source_point).line_to(target_point)
}

/// Draws a straight step path from the step points.
pub fn draw_straight_step_path(builder: StepsBuilder) -> ShapePath {
    let StepsBuilder { points, .. } = builder;
    points
        .into_iter()
        .fold(ShapePath::new(), |path, point| path.line_to(point))
}

/// Draws a smooth step path from the step points.
pub fn draw_smooth_step_path(builder: StepsBuilder) -> ShapePath {
    builder
        .smooth()
        .into_iter()
        .fold(ShapePath::new(), |path, action| match action {
            Action::MoveTo(to) => path.move_to(to),
            Action::LineTo(to) => path.line_to(to),
            Action::ArcTo(center, radii, sweep_angle) => path.arc(center, radii, sweep_angle, 0.0),
            Action::CubicBezierTo(ctrl1, ctrl2, to) => path.cubic_bezier_to(ctrl1, ctrl2, to),
        })
}
