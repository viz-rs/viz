use std::{future::Future, pin::Pin, time::Duration};

use tokio::time::timeout;

use viz_core::{http, Context, Middleware, Response, Result};

use viz_utils::tracing;

/// Timeout Middleware
#[derive(Debug)]
pub struct TimeoutMiddleware {
    /// 0.256s
    delay: Duration,
}

impl TimeoutMiddleware {
    /// Creates Timeout Middleware
    pub fn new(delay: Duration) -> Self {
        Self { delay }
    }
}

impl Default for TimeoutMiddleware {
    fn default() -> Self {
        Self::new(Duration::from_millis(256))
    }
}

impl TimeoutMiddleware {
    #[tracing::instrument(skip(cx))]
    async fn run(&self, cx: &mut Context) -> Result<Response> {
        let method = cx.method().to_owned();
        let path = cx.path().to_owned();

        match timeout(self.delay, cx.next()).await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("Timeout: {} {} {}", method, path, e);
                Ok(http::StatusCode::REQUEST_TIMEOUT.into())
            }
        }
    }
}

impl<'a> Middleware<'a, Context> for TimeoutMiddleware {
    type Output = Result<Response>;

    #[must_use]
    fn call(
        &'a self,
        cx: &'a mut Context,
    ) -> Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>> {
        Box::pin(self.run(cx))
    }
}
