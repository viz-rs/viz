use crate::{Handler, IntoResponse, Response, Result};

/// Maps the handler's output type to the [`Response`].
#[derive(Clone, Debug)]
pub struct MapInToResponse<H>(pub(crate) H);

impl<H> MapInToResponse<H> {
    /// Creates a [`MapInToResponse`] handler.
    #[inline]
    pub const fn new(h: H) -> Self {
        Self(h)
    }
}

#[crate::async_trait]
impl<H, I, O> Handler<I> for MapInToResponse<H>
where
    I: Send + 'static,
    H: Handler<I, Output = Result<O>>,
    O: IntoResponse,
{
    type Output = Result<Response>;

    async fn call(&self, i: I) -> Self::Output {
        self.0.call(i).await.map(IntoResponse::into_response)
    }
}
