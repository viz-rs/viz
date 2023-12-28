//! [OpenTelemetry(OTEL) Prometheus Exporter][OTEL].
//!
//! [OTEL]: https://docs.rs/opentelemetry-prometheus

use http_body_util::Full;
use opentelemetry::{global::handle_error, metrics::MetricsError};
use prometheus::{Encoder, TextEncoder};

use viz_core::{
    future::BoxFuture,
    header::{HeaderValue, CONTENT_TYPE},
    Handler, IntoResponse, Request, Response, Result, StatusCode,
};

#[doc(inline)]
pub use opentelemetry_prometheus::ExporterBuilder;
#[doc(inline)]
pub use prometheus::Registry;

/// The [`Registry`] wrapper.
#[derive(Clone, Debug)]
pub struct Prometheus {
    registry: Registry,
}

impl Prometheus {
    /// Creates a new [`Prometheus`].
    #[must_use]
    pub fn new(registry: Registry) -> Self {
        Self { registry }
    }
}

impl Handler<Request> for Prometheus {
    type Output = Result<Response>;

    fn call(&self, _: Request) -> BoxFuture<'static, Self::Output> {
        let Self { registry } = self.clone();

        Box::pin(async move {
            let metric_families = registry.gather();
            let encoder = TextEncoder::new();
            let mut body = Vec::new();

            if let Err(err) = encoder.encode(&metric_families, &mut body) {
                let text = err.to_string();
                handle_error(MetricsError::Other(text.clone()));
                Err((StatusCode::INTERNAL_SERVER_ERROR, text).into_error())?;
            }

            let mut res = Response::new(Full::from(body).into());

            res.headers_mut().append(
                CONTENT_TYPE,
                HeaderValue::from_str(encoder.format_type()).map_err(|err| {
                    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_error()
                })?,
            );

            Ok(res)
        })
    }
}
