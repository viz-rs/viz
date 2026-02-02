use bevy_math::Vec2;
use bevy_prototype_lyon::prelude::Geometry;
use flowkit::{
    edge::{EdgePosition, EdgeType},
    path::PathBuilder,
};
use lyon_path::{BuilderImpl, builder::WithSvg};

pub mod prelude {
    pub use flowkit::corner::{Corner, CornerPathParams};
    pub use flowkit::edge::{EdgePosition, EdgeType};
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

impl Geometry<WithSvg<BuilderImpl>> for EdgePath {
    fn add_geometry(&self, builder: &mut WithSvg<BuilderImpl>) {
        let &Self {
            source,
            target,
            edge_type,
            curvature,
            offset,
        } = self;
        PathBuilder::new(source, target, edge_type, curvature, offset).with(builder);
    }
}
