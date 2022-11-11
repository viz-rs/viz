//! Compression Middleware.

use std::str::FromStr;

use async_compression::tokio::bufread;
use tokio_util::io::{ReaderStream, StreamReader};

use crate::{
    async_trait,
    header::{HeaderValue, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH},
    Handler, IntoResponse, OutgoingBody, Request, Response, Result, Transform,
};

/// Compress response body.
#[derive(Debug, Default)]
pub struct Config;

impl<H> Transform<H> for Config
where
    H: Clone,
{
    type Output = CompressionMiddleware<H>;

    fn transform(&self, h: H) -> Self::Output {
        CompressionMiddleware { h }
    }
}

/// Compression middleware.
#[derive(Clone, Debug)]
pub struct CompressionMiddleware<H> {
    h: H,
}

#[async_trait]
impl<H, O> Handler<Request> for CompressionMiddleware<H>
where
    O: IntoResponse,
    H: Handler<Request, Output = Result<O>> + Clone,
{
    type Output = Result<Response>;

    async fn call(&self, req: Request) -> Self::Output {
        let accept_encoding = req
            .headers()
            .get(ACCEPT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_accept_encoding);

        let raw = self.h.call(req).await?;

        Ok(match accept_encoding {
            Some(algo) => Compress::new(raw, algo).into_response(),
            None => raw.into_response(),
        })
    }
}

/// Compresses the response body with the specified algorithm
/// and sets the `Content-Encoding` header.
#[derive(Debug)]
pub struct Compress<T> {
    inner: T,
    algo: ContentCoding,
}

impl<T> Compress<T> {
    /// Creates a compressed response with the specified algorithm.
    pub fn new(inner: T, algo: ContentCoding) -> Self {
        Self { inner, algo }
    }
}

impl<T: IntoResponse> IntoResponse for Compress<T> {
    fn into_response(self) -> Response {
        let mut res = self.inner.into_response();

        match self.algo {
            ContentCoding::Gzip | ContentCoding::Deflate | ContentCoding::Brotli => {
                res = res.map(|body| {
                    let body = StreamReader::new(body);
                    if self.algo == ContentCoding::Gzip {
                        OutgoingBody::streaming(ReaderStream::new(bufread::GzipEncoder::new(body)))
                    } else if self.algo == ContentCoding::Deflate {
                        OutgoingBody::streaming(ReaderStream::new(bufread::DeflateEncoder::new(
                            body,
                        )))
                    } else {
                        OutgoingBody::streaming(ReaderStream::new(bufread::BrotliEncoder::new(
                            body,
                        )))
                    }
                });
                res.headers_mut()
                    .append(CONTENT_ENCODING, HeaderValue::from_static(self.algo.into()));
                res.headers_mut().remove(CONTENT_LENGTH);
                res
            }
            ContentCoding::Any => res,
        }
    }
}

/// [ContentCoding]
///
/// [ContentCoding]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept-Encoding
#[derive(Debug, PartialEq)]
pub enum ContentCoding {
    /// gzip
    Gzip,
    /// deflate
    Deflate,
    /// brotli
    Brotli,
    /// *
    Any,
}

impl FromStr for ContentCoding {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("deflate") {
            Ok(ContentCoding::Deflate)
        } else if s.eq_ignore_ascii_case("gzip") {
            Ok(ContentCoding::Gzip)
        } else if s.eq_ignore_ascii_case("br") {
            Ok(ContentCoding::Brotli)
        } else if s == "*" {
            Ok(ContentCoding::Any)
        } else {
            Err(())
        }
    }
}

impl From<ContentCoding> for &'static str {
    fn from(cc: ContentCoding) -> Self {
        match cc {
            ContentCoding::Gzip => "gzip",
            ContentCoding::Deflate => "deflate",
            ContentCoding::Brotli => "br",
            ContentCoding::Any => "*",
        }
    }
}

fn parse_accept_encoding(s: &str) -> Option<ContentCoding> {
    s.split(',')
        .map(str::trim)
        .filter_map(|v| {
            Some(match v.split_once(";q=") {
                Some((c, q)) => (
                    c.parse::<ContentCoding>().ok()?,
                    q.parse::<f32>().ok()? * 1000.,
                ),
                None => (v.parse::<ContentCoding>().ok()?, 1000.),
            })
        })
        .max_by_key(|(_, q)| *q as u16)
        .map(|(c, _)| c)
}
