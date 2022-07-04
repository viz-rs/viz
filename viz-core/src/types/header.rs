use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use crate::{
    async_trait, header,
    headers::{self, HeaderMapExt},
    Body, FromRequest, IntoResponse, Request, Result, StatusCode, ThisError,
};

/// Header Extractor
pub struct Header<T: ?Sized>(pub T);

impl<T> Header<T> {
    /// Create new `Data` instance.
    #[inline]
    pub fn new(t: T) -> Self {
        Self(t)
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Clone for Header<T>
where
    T: ?Sized + Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> AsRef<T> for Header<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> Deref for Header<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Header<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> fmt::Debug for Header<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(self, f)
    }
}

#[derive(ThisError, Debug)]
pub enum HeaderError {
    #[error("Invalid header name {0}")]
    InvalidName(&'static header::HeaderName),
    #[error("Missing header name {0}")]
    MissingName(&'static header::HeaderName),
    #[error("Invalid header value {0}")]
    InvalidValue(header::InvalidHeaderValue),
}

impl IntoResponse for HeaderError {
    fn into_response(self) -> http::Response<Body> {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

#[async_trait]
impl<T> FromRequest for Header<T>
where
    T: headers::Header,
{
    type Error = HeaderError;

    async fn extract(req: &mut Request<Body>) -> Result<Self, Self::Error> {
        req.headers()
            .typed_try_get::<T>()
            .map_err(|_| HeaderError::InvalidName(T::name()))
            .and_then(|v| v.ok_or_else(|| HeaderError::MissingName(T::name())))
            .map(Self)
    }
}
