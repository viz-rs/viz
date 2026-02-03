use egui::{
    Color32, Shape,
    epaint::{CubicBezierShape, QuadraticBezierShape},
};
use flowkit::path::PathBuilder;
use lyon_path::{BuilderImpl, Event, builder::WithSvg};

pub use lyon_tessellation::StrokeOptions;

pub use flowkit::corner::{Corner, CornerPathParams};
pub use flowkit::edge::EdgeType;

// Y-axis should be down.
pub type EdgeAnchor = flowkit::edge::EdgeAnchor<false>;
pub type EdgePoint = flowkit::edge::EdgePoint<false>;
pub type EdgePath = flowkit::edge::EdgePath<false>;
pub type Pathbuilder = flowkit::path::PathBuilder<false>;

use crate::{
    mesh::{Mode, Tessellator},
    utils::Convert,
    vertex::VertexBuffers,
};

mod utils;

pub mod mesh;
pub mod vertex;

/// Draws a connection, the `EdgePath` wrapper.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Connection(EdgePath);

impl Default for Connection {
    fn default() -> Self {
        Self(EdgePath::DEFAULT)
    }
}

impl From<EdgePath> for Connection {
    fn from(path: EdgePath) -> Self {
        Self(path)
    }
}

impl Connection {
    pub fn build(self, stroke: impl Into<egui::Stroke>) -> Shape {
        const FILL: Color32 = Color32::TRANSPARENT;
        let stroke = stroke.into();
        let edge_type = self.0.edge_type;

        let internal_builder = PathBuilder::from(self.0);
        let builder: WithSvg<BuilderImpl> = internal_builder.into();
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
        let internal_builder = PathBuilder::from(self.0);
        let builder: WithSvg<BuilderImpl> = internal_builder.into();
        let path = builder.build();

        let mut buffers = VertexBuffers::new();

        tess.stroke(&path, mode, &mut buffers);

        let mesh = Tessellator::build_mesh(buffers);
        Shape::mesh(::std::sync::Arc::new(mesh))
    }
}
