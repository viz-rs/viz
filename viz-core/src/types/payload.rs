//! Request Payload Trait and Payload Error.

use crate::{Error, IntoResponse, Response, Result, StatusCode, ThisError};

/// Rejects with an error when the body of request extraction fails.
#[derive(ThisError, Debug)]
pub enum PayloadError {
    /// 400
    #[error("failed to read payload")]
    Read,

    /// 400
    #[error("failed to parse payload")]
    Parse,

    /// 400
    #[error("multipart missing boundary")]
    MissingBoundary,

    /// 400
    #[error("parse utf8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// 400
    #[error("{0}")]
    Hyper(#[from] hyper::Error),

    #[cfg(feature = "json")]
    /// 400
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[cfg(any(feature = "form", feature = "query"))]
    /// 400
    #[error("url decode: {0}")]
    UrlDecode(#[from] serde_urlencoded::de::Error),

    /// 411
    #[error("content-length is required")]
    LengthRequired,

    /// 413
    #[error("payload is too large")]
    TooLarge,

    /// 415
    #[error("unsupported media type, `{}` is required", .0.to_string())]
    UnsupportedMediaType(mime::Mime),

    /// 500
    #[error("payload has been used")]
    Used,
}

impl IntoResponse for PayloadError {
    fn into_response(self) -> Response {
        (
            match self {
                PayloadError::Read
                | PayloadError::Parse
                | PayloadError::MissingBoundary
                | PayloadError::Utf8(_)
                | PayloadError::Hyper(_) => StatusCode::BAD_REQUEST,
                #[cfg(feature = "json")]
                PayloadError::Json(_) => StatusCode::BAD_REQUEST,
                #[cfg(any(feature = "form", feature = "query"))]
                PayloadError::UrlDecode(_) => StatusCode::BAD_REQUEST,
                PayloadError::LengthRequired => StatusCode::LENGTH_REQUIRED,
                PayloadError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
                PayloadError::UnsupportedMediaType(_) => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                PayloadError::Used => StatusCode::INTERNAL_SERVER_ERROR,
            },
            self.to_string(),
        )
            .into_response()
    }
}

impl From<PayloadError> for Error {
    fn from(e: PayloadError) -> Self {
        e.into_error()
    }
}

/// An interface for processing the payload data of the HTTP request.
pub trait Payload {
    /// Named the payload.
    const NAME: &'static str = "payload";

    /// Limited the payload data size, by default 1MB.
    const LIMIT: u64 = 1024 * 1024;

    /// Specified a media type.
    fn mime() -> mime::Mime;

    /// Detects the payload media type.
    fn detect(m: &mime::Mime) -> bool;

    /// Sets the limit size.
    #[must_use]
    #[inline]
    fn limit(limit: Option<u64>) -> u64 {
        limit.unwrap_or(Self::LIMIT)
    }

    /// Checks `Content-Type` & `Content-Length`
    ///
    /// # Errors
    ///
    /// 1. unsupported media type
    /// 2. content-length is required
    /// 3. payload is too large
    #[inline]
    fn check_header(
        m: Option<mime::Mime>,
        len: Option<u64>,
        limit: Option<u64>,
    ) -> Result<mime::Mime, PayloadError> {
        let m = m.ok_or_else(|| PayloadError::UnsupportedMediaType(Self::mime()))?;

        if !Self::detect(&m) {
            return Err(PayloadError::UnsupportedMediaType(Self::mime()));
        }

        if len.is_none() {
            return Err(PayloadError::LengthRequired);
        }

        if matches!(len, Some(len) if len  > Self::limit(limit)) {
            return Err(PayloadError::TooLarge);
        }

        Ok(m)
    }
}
