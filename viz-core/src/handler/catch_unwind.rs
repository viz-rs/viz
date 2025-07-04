use crate::{Handler, IntoResponse, Response, Result, future::FutureExt};

/// Catches unwinding panics while calling the handler.
#[derive(Clone, Debug)]
pub struct CatchUnwind<H, F> {
    h: H,
    f: F,
}

impl<H, F> CatchUnwind<H, F> {
    /// Creates an [`CatchUnwind`] handler.
    #[inline]
    pub const fn new(h: H, f: F) -> Self {
        Self { h, f }
    }
}

#[crate::async_trait]
impl<H, F, I, O, R> Handler<I> for CatchUnwind<H, F>
where
    I: Send + 'static,
    H: Handler<I, Output = Result<O>>,
    O: IntoResponse + Send,
    F: Handler<Box<dyn ::core::any::Any + Send>, Output = R>,
    R: IntoResponse,
{
    type Output = Result<Response>;

    async fn call(&self, i: I) -> Self::Output {
        match ::core::panic::AssertUnwindSafe(self.h.call(i))
            .catch_unwind()
            .await
        {
            Ok(r) => r.map(IntoResponse::into_response),
            Err(e) => Ok(self.f.call(e).await.into_response()),
        }
    }
}
