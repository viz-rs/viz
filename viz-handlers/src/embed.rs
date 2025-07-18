//! Static files serving and embedding.

use std::{borrow::Cow, marker::PhantomData};

use http_body_util::Full;
use rust_embed::{EmbeddedFile, RustEmbed};
use viz_core::{
    Handler, IntoResponse, Method, Request, RequestExt, Response, Result, StatusCode,
    header::{CONTENT_TYPE, ETAG, IF_NONE_MATCH},
};

/// Serve a single embedded file.
#[derive(Debug)]
pub struct File<E>(Cow<'static, str>, PhantomData<E>);

impl<E> Clone for File<E> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}

impl<E> File<E> {
    /// Serve a new file by the specified path.
    #[must_use]
    pub fn new(path: &'static str) -> Self {
        Self(path.into(), PhantomData)
    }
}

#[viz_core::async_trait]
impl<E> Handler<Request> for File<E>
where
    E: RustEmbed + Send + Sync + 'static,
{
    type Output = Result<Response>;

    async fn call(&self, req: Request) -> Self::Output {
        serve::<E>(&self.0, &req)
    }
}

/// Serve a embedded directory.
#[derive(Debug)]
pub struct Dir<E>(PhantomData<E>);

impl<E> Clone for Dir<E> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<E> Default for Dir<E> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[viz_core::async_trait]
impl<E> Handler<Request> for Dir<E>
where
    E: RustEmbed + Send + Sync + 'static,
{
    type Output = Result<Response>;

    async fn call(&self, req: Request) -> Self::Output {
        serve::<E>(
            req.route_info()
                .params
                .first()
                .map(|(_, v)| v)
                .map_or("index.html", |p| p),
            &req,
        )
    }
}

fn serve<E>(path: &str, req: &Request) -> Result<Response>
where
    E: RustEmbed + Send + Sync + 'static,
{
    if Method::GET != req.method() {
        Err(StatusCode::METHOD_NOT_ALLOWED.into_error())?;
    }

    match E::get(path) {
        Some(EmbeddedFile { data, metadata }) => {
            let hash = hex::encode(metadata.sha256_hash());

            if req
                .headers()
                .get(IF_NONE_MATCH)
                .is_some_and(|etag| etag.to_str().unwrap_or("000000").eq(&hash))
            {
                Err(StatusCode::NOT_MODIFIED.into_error())?;
            }

            Response::builder()
                .header(
                    CONTENT_TYPE,
                    mime_guess::from_path(path).first_or_octet_stream().as_ref(),
                )
                .header(ETAG, hash)
                .body(Full::from(data).into())
                .map_err(Into::into)
        }
        None => Err(StatusCode::NOT_FOUND.into_error()),
    }
}
