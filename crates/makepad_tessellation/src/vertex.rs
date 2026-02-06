use lyon_tessellation::{
    self as tess, FillVertex, FillVertexConstructor, StrokeVertex, StrokeVertexConstructor,
};
use makepad_vector::geometry::Point;

use crate::utils::Convert;

/// The index type of a mesh.
type IndexType = u32;
/// Lyon's [`VertexBuffers`] generic data type defined for [`Point`].
pub type VertexBuffers = tess::VertexBuffers<Point, IndexType>;

/// Zero-sized type used to implement various vertex construction traits from Lyon.
pub struct VertexConstructor;

/// Enables the construction of a [`Point`] when using a `FillTessellator`.
impl FillVertexConstructor<Point> for VertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> Point {
        vertex.position().convert()
    }
}

/// Enables the construction of a [`Point`] when using a `StrokeTessellator`.
impl StrokeVertexConstructor<Point> for VertexConstructor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> Point {
        vertex.position().convert()
    }
}
