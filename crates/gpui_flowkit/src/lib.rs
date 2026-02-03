use lyon_path::BuilderImpl;

pub use flowkit::corner::{Corner, CornerPathParams};
pub use flowkit::edge::EdgeType;
use lyon_path::builder::WithSvg;

// Y-axis should be down.
pub type EdgeAnchor = flowkit::edge::EdgeAnchor<false>;
pub type EdgePoint = flowkit::edge::EdgePoint<false>;
pub type EdgePath = flowkit::edge::EdgePath<false>;
pub type PathBuilder = flowkit::path::PathBuilder<false>;

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
    fn from(conn: Connection) -> Self {
        let internal_builder = PathBuilder::from(conn.0);
        let builder: WithSvg<BuilderImpl> = internal_builder.into();
        builder.into()
    }
}
