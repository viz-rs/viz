#[cfg(feature = "json")]
use bytes::{BufMut, BytesMut};
use http_body_util::Full;

use crate::{header, Bytes, Error, OutgoingBody, Response, Result, StatusCode};

/// The [Response] Extension.
pub trait ResponseExt: Sized {
    /// Get the size of this response's body.
    fn content_length(&self) -> Option<u64>;

    /// The response with the specified [`Content-Type`][mdn].
    ///
    /// [mdn]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Type>
    fn with<B>(body: B, content_type: &'static str) -> Response
    where
        B: Into<OutgoingBody>,
    {
        let mut res = Response::new(body.into());
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(content_type),
        );
        res
    }

    /// The response with `text/plain; charset=utf-8` media type.
    fn text<B>(body: B) -> Response
    where
        B: Into<Full<Bytes>>,
    {
        Self::with(body.into(), mime::TEXT_PLAIN_UTF_8.as_ref())
    }

    /// The response with `text/html; charset=utf-8` media type.
    fn html<B>(body: B) -> Response
    where
        B: Into<Full<Bytes>>,
    {
        Self::with(body.into(), mime::TEXT_HTML_UTF_8.as_ref())
    }

    #[cfg(feature = "json")]
    /// The response with `application/javascript; charset=utf-8` media type.
    ///
    /// # Errors
    ///
    /// Throws an error if serialization fails.
    fn json<T>(body: T) -> Result<Response, crate::types::PayloadError>
    where
        T: serde::Serialize,
    {
        let mut buf = BytesMut::new().writer();
        serde_json::to_writer(&mut buf, &body)
            .map(|_| {
                Self::with(
                    Full::new(buf.into_inner().freeze()),
                    mime::APPLICATION_JSON.as_ref(),
                )
            })
            .map_err(crate::types::PayloadError::Json)
    }

    /// Responds to a stream.
    fn stream<S, D, E>(stream: S) -> Response
    where
        S: futures_util::Stream<Item = Result<D, E>> + Send + Sync + 'static,
        D: Into<Bytes>,
        E: Into<Error> + 'static,
    {
        Response::new(OutgoingBody::streaming(stream))
    }

    // TODO: Download transfers the file from path as an attachment.
    // fn download() -> Response<Body>

    /// The response was successful (status in the range [`200-299`][mdn]) or not.
    ///
    /// [mdn]: <https://developer.mozilla.org/en-US/docs/Web/API/Response/ok>
    fn ok(&self) -> bool;

    /// The [`Content-Disposition`][mdn] header indicates if the content is expected to be
    /// displayed inline in the browser, that is, as a Web page or as part of a Web page,
    /// or as an attachment, that is downloaded and saved locally.
    ///
    /// [mdn]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Disposition>
    fn attachment(file: &str) -> Self;

    /// The [`Content-Location`][mdn] header indicates an alternate location for the returned data.
    ///
    /// [mdn]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Location>
    fn location<T>(location: T) -> Self
    where
        T: AsRef<str>;

    /// The response redirects to the specified URL.
    ///
    /// [mdn]: <https://developer.mozilla.org/en-US/docs/Web/API/Response/redirect>
    fn redirect<T>(url: T) -> Self
    where
        T: AsRef<str>;

    /// The response redirects to the specified URL and the status code.
    ///
    /// [mdn]: <https://developer.mozilla.org/en-US/docs/Web/API/Response/redirect>
    fn redirect_with_status<T>(uri: T, status: StatusCode) -> Self
    where
        T: AsRef<str>;

    /// The response redirects to the [`303`][mdn].
    ///
    /// [mdn]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/303>
    fn see_other<T>(url: T) -> Self
    where
        T: AsRef<str>,
    {
        Self::redirect_with_status(url, StatusCode::SEE_OTHER)
    }

    /// The response redirects to the [`307`][mdn].
    ///
    /// [mdn]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/307>
    fn temporary<T>(url: T) -> Self
    where
        T: AsRef<str>,
    {
        Self::redirect_with_status(url, StatusCode::TEMPORARY_REDIRECT)
    }

    /// The response redirects to the [`308`][mdn].
    ///
    /// [mdn]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/308>
    fn permanent<T>(url: T) -> Self
    where
        T: AsRef<str>,
    {
        Self::redirect_with_status(url, StatusCode::PERMANENT_REDIRECT)
    }
}

impl ResponseExt for Response {
    fn content_length(&self) -> Option<u64> {
        self.headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
    }

    fn ok(&self) -> bool {
        self.status().is_success()
    }

    fn attachment(file: &str) -> Self {
        let mut res = Self::default();
        let val = header::HeaderValue::from_str(file)
            .expect("content-disposition is not the correct value");
        res.headers_mut().insert(header::CONTENT_DISPOSITION, val);
        res
    }

    fn location<T>(location: T) -> Self
    where
        T: AsRef<str>,
    {
        let val = header::HeaderValue::try_from(location.as_ref())
            .expect("location is not the correct value");
        let mut res = Self::default();
        res.headers_mut().insert(header::CONTENT_LOCATION, val);
        res
    }

    fn redirect<T>(url: T) -> Self
    where
        T: AsRef<str>,
    {
        let val =
            header::HeaderValue::try_from(url.as_ref()).expect("url is not the correct value");
        let mut res = Self::default();
        res.headers_mut().insert(header::LOCATION, val);
        res
    }

    fn redirect_with_status<T>(url: T, status: StatusCode) -> Self
    where
        T: AsRef<str>,
    {
        assert!(status.is_redirection(), "not a redirection status code");

        let mut res = Self::redirect(url);
        *res.status_mut() = status;
        res
    }
}
