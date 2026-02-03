use flowkit::{CURVATURE, OFFSET, edge::EdgeType, path::PathBuilder};
use glam::Vec2;
use lyon_path::BuilderImpl;

pub type EdgePosition = flowkit::edge::EdgePosition<false>;

pub mod prelude {
    pub use super::EdgePosition;
    pub use flowkit::corner::{Corner, CornerPathParams};
    pub use flowkit::edge::EdgeType;
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

    pub fn as_path_builder(&self) -> PathBuilder<false> {
        PathBuilder::new(
            self.source,
            self.target,
            self.edge_type,
            self.curvature,
            self.offset,
        )
    }
}

impl Into<gpui::PathBuilder> for EdgePath {
    fn into(self) -> gpui::PathBuilder {
        let mut builder = BuilderImpl::new().with_svg();

        self.as_path_builder().with(&mut builder);

        builder.into()
    }
}
