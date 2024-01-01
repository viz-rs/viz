use crate::{async_trait, Handler, Result};

/// Maps the `Ok` value of the output if after the handler called.
#[derive(Debug, Clone)]
pub struct Map<H, F> {
    h: H,
    f: F,
}

impl<H, F> Map<H, F> {
    /// Creates a [`Map`] handler.
    #[inline]
    pub fn new(h: H, f: F) -> Self {
        Self { h, f }
    }
}

#[async_trait]
impl<H, F, I, O, T> Handler<I> for Map<H, F>
where
    I: Send + 'static,
    H: Handler<I, Output = Result<O>>,
    F: FnOnce(O) -> T + Send + Sync + Copy + 'static,
{
    type Output = Result<T>;

    async fn call(&self, i: I) -> Self::Output {
        self.h.call(i).await.map(self.f)
    }
}
