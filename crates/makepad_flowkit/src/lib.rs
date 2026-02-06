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

        tess.stroke(path, mode, &mut buffers);

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

/// Draws a connection with indices and vertices.
///
/// How does it work?
///
/// 1. Extracts 4 vertices' positions from two triangles.
///
/// ```
/// ----------------- | ------------- | ------------
/// `quad_2d.indices` | `0 1 2 2 3 0` |
/// `buffers.indices` | `3 2 0 3 0 1` | `d c a d a b`
///
/// 0  1  a  b
///  []    []
/// 3  2  d  c
///
/// ----- | ------ | ----------- | -------
/// index | indice |   vertex    | corner
///   0   |   a    | vertices[a] | top-left
///   1   |   b    | vertices[b] | top-right
///   2   |   c    | vertices[c] | bottom-right
///   3   |   d    | vertices[d] | bottom-left
///
/// 2. Draws two lines:
/// a -> c
/// b -> d
/// ```
pub fn draw_with(
    cx: &mut Cx2d,
    draw_line: &mut DrawLine,
    buffers: VertexBuffers,
    color: Vec4,
    width: f64,
) {
    for indices in buffers.indices.chunks(6) {
        let [d, c, a, _, _, b] = indices[..] else {
            break;
        };

        let av = buffers.vertices[a as usize];
        let bv = buffers.vertices[b as usize];
        let cv = buffers.vertices[c as usize];
        let dv = buffers.vertices[d as usize];

        let ap = Vec2d { x: av.x, y: av.y };
        let bp = Vec2d { x: bv.x, y: bv.y };
        let cp = Vec2d { x: cv.x, y: cv.y };
        let dp = Vec2d { x: dv.x, y: dv.y };

        draw_line.draw_line_abs(cx, ap, cp, color, width);
        draw_line.draw_line_abs(cx, bp, dp, color, width);
    }
}
