use egui::{Color32, Mesh, TextureId};
use lyon_path::Path;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, StrokeOptions, StrokeTessellator,
};

use crate::vertex::{VertexBuffers, VertexConstructor};

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct Mode<T> {
    pub options: T,
    pub color: Color32,
}

/// A Tessellator resource, includes `FillTessellator` and `StrokeTessellator`.
pub struct Tessellator {
    fill: FillTessellator,
    stroke: StrokeTessellator,
}

impl Default for Tessellator {
    fn default() -> Self {
        Self {
            fill: FillTessellator::new(),
            stroke: StrokeTessellator::new(),
        }
    }
}

impl Tessellator {
    pub fn fill(&mut self, path: &Path, mode: Mode<FillOptions>, buffers: &mut VertexBuffers) {
        if let Err(e) = self.fill.tessellate_path(
            path,
            &mode.options,
            &mut BuffersBuilder::new(buffers, VertexConstructor { color: mode.color }),
        ) {
            tracing::error!("FillTessellator error: {:?}", e);
        }
    }

    pub fn stroke(&mut self, path: &Path, mode: Mode<StrokeOptions>, buffers: &mut VertexBuffers) {
        if let Err(e) = self.stroke.tessellate_path(
            path,
            &mode.options,
            &mut BuffersBuilder::new(buffers, VertexConstructor { color: mode.color }),
        ) {
            tracing::error!("StrokeTessellator error: {:?}", e);
        }
    }
}

pub fn build_mesh(buffers: VertexBuffers) -> Mesh {
    Mesh {
        indices: buffers.indices,
        vertices: buffers.vertices,
        texture_id: TextureId::default(),
    }
}
