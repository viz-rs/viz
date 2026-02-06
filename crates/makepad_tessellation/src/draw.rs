use makepad_widgets::{Cx2d, DrawLine, Vec4, dvec2};

use crate::vertex::VertexBuffers;

/// Draws a path with indices and vertices.
///
/// How does it work?
///
/// 1. Extracts 4 vertices' positions from two triangles.
///
/// ```
/// ----------------- | ------------- | ------------
/// `quad_2d.indices` | `0 1 2 2 3 0` | `a b c c d a`
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
///
/// `a -> c`
/// `b -> d`
/// ```
pub fn draw_with(
    cx: &mut Cx2d,
    draw_line: &mut DrawLine,
    buffers: VertexBuffers,
    color: Vec4,
    width: f64,
) {
    let VertexBuffers { indices, vertices } = buffers;

    for chunks in indices.chunks(6) {
        let [d, c, a, _, _, b] = chunks[..] else {
            break;
        };

        let av = vertices[a as usize];
        let bv = vertices[b as usize];
        let cv = vertices[c as usize];
        let dv = vertices[d as usize];

        let ap = dvec2(av.x, av.y);
        let bp = dvec2(bv.x, bv.y);
        let cp = dvec2(cv.x, cv.y);
        let dp = dvec2(dv.x, dv.y);

        draw_line.draw_line_abs(cx, ap, cp, color, width);
        draw_line.draw_line_abs(cx, bp, dp, color, width);
    }
}
