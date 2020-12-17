#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]

//! Viz Core

use viz_utils::anyhow;

pub use anyhow::Error;

/// Result
pub type Result<T = (), E = anyhow::Error> = anyhow::Result<T, E>;

mod context;
mod extract;
mod guard;
mod handler;
mod macros;
mod middleware;
mod response;
mod types;

pub mod config;

#[cfg(feature = "sse")]
pub mod sse;
#[cfg(feature = "ws")]
pub mod ws;

pub use context::Context;
pub use extract::Extract;
pub use guard::{into_guard, Guard};
pub use handler::{Handler, HandlerBase, HandlerCamp, HandlerSuper, HandlerWrapper};
pub use middleware::{DynMiddleware, Middleware, Middlewares};
pub use response::Response;
pub use types::*;

pub mod http {
    pub use ::http::*;
    pub use hyper::Body;

    pub type Request<T = Body> = ::http::Request<T>;
    pub type Response<T = Body> = ::http::Response<T>;
}

/// Responds a custom error to response.
#[macro_export]
macro_rules! reject {
    ($err:expr) => {
        return Err(how!($err));
    };
}

/// Converts a custom error to [`Response`] and then converts to [`Error`].
#[macro_export]
macro_rules! how {
    ($err:expr) => {
        Into::<Error>::into(Into::<Response>::into($err))
    };
}

pub use crate::anyhow::{anyhow, bail, ensure};
