use lyon_path::{BuilderImpl, builder::WithSvg};

pub use flowkit::{
    corner::{Corner, CornerPathParams},
    edge::{EdgeAnchor, EdgePath, EdgePoint, EdgeType},
    path::PathBuilder,
};

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

impl From<Connection> for gpui::PathBuilder {
    fn from(value: Connection) -> Self {
        let internal_builder = PathBuilder::from((value.0, true));
        let builder: WithSvg<BuilderImpl> = internal_builder.into();
        builder.into()
    }
}
