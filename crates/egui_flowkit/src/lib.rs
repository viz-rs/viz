use egui::{
    Color32, Shape,
    epaint::{CubicBezierShape, QuadraticBezierShape},
};
use flowkit::{
    CURVATURE, OFFSET,
    edge::{EdgePosition, EdgeType},
    path::PathBuilder,
};
use glam::Vec2;
use lyon_path::{BuilderImpl, Event};
pub use lyon_tessellation::StrokeOptions;

use crate::{
    mesh::{Mode, Tessellator},
    utils::Convert,
    vertex::VertexBuffers,
};

mod utils;

pub mod mesh;
pub mod vertex;

pub mod prelude {
    pub use flowkit::corner::{Corner, CornerPathParams};
    pub use flowkit::edge::{EdgePosition, EdgeType};
    pub use flowkit::{CURVATURE, OFFSET};
}

/// Draws an edge path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgePath {
    pub source: (Vec2, EdgePosition),
    pub target: (Vec2, EdgePosition),
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
        source: (Vec2::ZERO, EdgePosition::Right),
        target: (Vec2::ZERO, EdgePosition::Left),
        edge_type: EdgeType::Straight,
        curvature: CURVATURE,
        offset: OFFSET,
    };

    pub fn build(self, stroke: impl Into<egui::Stroke>) -> Shape {
        let Self {
            source,
            target,
            edge_type,
            curvature,
            offset,
        } = self;

        const FILL: Color32 = Color32::TRANSPARENT;
        let stroke = stroke.into();

        let mut builder = BuilderImpl::new().with_svg();

        PathBuilder::new(source, target, edge_type, curvature, offset).with(&mut builder);

        let path = builder.build();

        let mut events = path.iter().filter(|&e| match e {
            Event::Begin { .. } | Event::End { .. } => false,
            _ => true,
        });

        match edge_type {
            EdgeType::Straight => {
                if let Some(Event::Line { from, to }) = events.next() {
                    return Shape::line_segment([from, to].convert(), stroke);
                }
            }
            EdgeType::StraightStep => {
                let mut points = Vec::new();
                while let Some(Event::Line { from, to }) = events.next() {
                    points.extend([from, to].convert());
                }
                return Shape::line(points, stroke);
            }
            EdgeType::SmoothStep => {
                let mut shapes = Vec::new();
                for event in events {
                    match event {
                        Event::Line { from, to } => {
                            shapes.push(Shape::line_segment([from, to].convert(), stroke));
                        }
                        Event::Quadratic { from, ctrl, to } => {
                            shapes.push(Shape::QuadraticBezier(
                                QuadraticBezierShape::from_points_stroke(
                                    [from, ctrl, to].convert(),
                                    false,
                                    FILL,
                                    stroke,
                                ),
                            ));
                        }
                        Event::Cubic {
                            from,
                            ctrl1,
                            ctrl2,
                            to,
                        } => {
                            shapes.push(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                                [from, ctrl1, ctrl2, to].convert(),
                                false,
                                FILL,
                                stroke,
                            )));
                        }
                        _ => {
                            // do nothing
                        }
                    }
                }
                return shapes.into();
            }
            EdgeType::Curve => {
                if let Some(Event::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to,
                }) = events.next()
                {
                    return Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                        [from, ctrl1, ctrl2, to].convert(),
                        false,
                        FILL,
                        stroke,
                    ));
                }
            }
        }

        Shape::Noop
    }

    pub fn build_with(self, mode: Mode<StrokeOptions>, tess: &mut Tessellator) -> Shape {
        let Self {
            source,
            target,
            edge_type,
            curvature,
            offset,
        } = self;

        let mut builder = BuilderImpl::new().with_svg();

        PathBuilder::new(source, target, edge_type, curvature, offset).with(&mut builder);

        let path = builder.build();

        let mut buffers = VertexBuffers::new();

        tess.stroke(&path, mode, &mut buffers);

        let mesh = Tessellator::build_mesh(buffers);
        Shape::mesh(::std::sync::Arc::new(mesh))
    }
}
