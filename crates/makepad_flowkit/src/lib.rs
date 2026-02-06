use lyon_path::{BuilderImpl, Event, builder::WithSvg};
use makepad_vector::path::Path;
use makepad_widgets::{Cx2d, DrawLine, Vec2d, Vec4};

pub use lyon_tessellation::{StrokeOptions, StrokeTessellator};

pub use flowkit::{
    corner::{Corner, CornerPathParams},
    edge::{EdgeAnchor, EdgePath, EdgePoint, EdgeType},
    path::PathBuilder,
};

use crate::{
    mesh::{Mode, Tessellator},
    utils::Convert,
    vertex::VertexBuffers,
};

pub mod mesh;
pub mod utils;
pub mod vertex;

/// The `EdgePath` wrapper for drawing the connection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Connection(EdgePath);

impl Connection {
    pub fn build_with(
        self,
        mode: Mode<StrokeOptions>,
        tess: &mut Tessellator<StrokeTessellator>,
    ) -> VertexBuffers {
        let builder: WithSvg<BuilderImpl> = PathBuilder::from((self.0, true)).into();
        let path = builder.build();

        let mut buffers = VertexBuffers::new();

        tess.stroke(&path, mode, &mut buffers);

        buffers
    }
}

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

impl From<Connection> for Path {
    fn from(value: Connection) -> Self {
        let builder: WithSvg<BuilderImpl> = PathBuilder::from((value.0, true)).into();

        let events = builder.build();

        let mut path = Self::new();

        for event in &events {
            match event {
                Event::Line { from, to } => {
                    let [from, to] = [from, to].convert();
                    path.move_to(from);
                    path.line_to(to);
                }
                Event::Quadratic { from, ctrl, to } => {
                    let [from, ctrl, to] = [from, ctrl, to].convert();
                    path.move_to(from);
                    path.quadratic_to(ctrl, to);
                }
                Event::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to,
                } => {
                    let [from, ctrl1, ctrl2, to] = [from, ctrl1, ctrl2, to].convert();
                    path.move_to(from);
                    path.cubic_to(ctrl1, ctrl2, to);
                }
                _ => {
                    // do nothing
                }
            }
        }

        path.commands();

        path
    }
}

pub fn draw_with(
    cx: &mut Cx2d,
    draw_line: &mut DrawLine,
    buffers: VertexBuffers,
    color: Vec4,
    width: f64,
) {
    // c  f   0 1
    // a  b   3 2
    // a b c d e f
    // 3 2 0 3 0 1: buffers.indices order
    // 0 1 2 2 3 0: quad_2d.indices order
    // gets 4 corners:
    // c: top-left
    // f: top-right
    // a: bottom-left
    // b: bottom-right
    // draw line from `top-right` to `bottom-left`: a -> f
    // draw line from `top-left` to `bottom-right`: c -> b
    for i in buffers.indices.chunks(6) {
        let a = &buffers.vertices[i[0] as usize];
        let b = &buffers.vertices[i[1] as usize];
        let c = &buffers.vertices[i[2] as usize];
        // let d = &buffers.vertices[i[3] as usize];
        // let e = &buffers.vertices[i[4] as usize];
        let f = &buffers.vertices[i[5] as usize];

        let a = Vec2d {
            x: a.pos.x,
            y: a.pos.y,
        };
        let b = Vec2d {
            x: b.pos.x,
            y: b.pos.y,
        };
        let c = Vec2d {
            x: c.pos.x,
            y: c.pos.y,
        };
        // let d = Vec2d {
        //     x: d.pos.x,
        //     y: d.pos.y,
        // };
        // let e = Vec2d {
        //     x: e.pos.x,
        //     y: e.pos.y,
        // };
        let f = Vec2d {
            x: f.pos.x,
            y: f.pos.y,
        };

        draw_line.draw_line_abs(cx, a, f, color, width);

        draw_line.draw_line_abs(cx, c, b, color, width);
    }
}
