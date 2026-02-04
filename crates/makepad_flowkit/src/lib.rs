use lyon_path::{BuilderImpl, Event, builder::WithSvg};
use makepad_widgets::makepad_vector::path::Path;

pub use flowkit::{
    corner::{Corner, CornerPathParams},
    edge::{EdgeAnchor, EdgePath, EdgePoint, EdgeType},
    path::PathBuilder,
};

use crate::utils::Convert;

pub mod utils;

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

impl From<Connection> for Path {
    fn from(value: Connection) -> Self {
        let internal_builder = PathBuilder::from((value.0, true));
        let builder: WithSvg<BuilderImpl> = internal_builder.into();

        let events = builder.build();

        let mut path = Self::new();

        for event in &events {
            match event {
                Event::Line { from, to } => {
                    path.move_to(from.convert());
                    path.line_to(to.convert());
                }
                Event::Quadratic { from, ctrl, to } => {
                    path.move_to(from.convert());
                    path.quadratic_to(ctrl.convert(), to.convert());
                }
                Event::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to,
                } => {
                    path.move_to(from.convert());
                    path.cubic_to(ctrl1.convert(), ctrl2.convert(), to.convert());
                }
                _ => {
                    // do nothing
                }
            }
        }

        path
    }
}
