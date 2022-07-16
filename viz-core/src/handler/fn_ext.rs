use std::marker::PhantomData;

use crate::{async_trait, FromRequest, Handler, IntoResponse, Request, Response, Result};

/// Fn Extractor Trait
#[async_trait]
pub trait FnExt<Args>: Clone + Send + Sync + 'static {
    type Output;

    async fn call(&self, req: Request) -> Self::Output;
}

/// Responder with extractor
pub struct ResponderExt<H, O, I = ()>(H, PhantomData<O>, PhantomData<I>);

impl<H, O, I> Clone for ResponderExt<H, O, I>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData, PhantomData)
    }
}

impl<H, O, I> ResponderExt<H, O, I> {
    pub fn new(h: H) -> Self {
        Self(h, PhantomData, PhantomData)
    }
}

#[async_trait]
impl<H, O, I> Handler<Request> for ResponderExt<H, O, I>
where
    I: FromRequest + Send + Sync + 'static,
    I::Error: IntoResponse + Send + Sync,
    H: FnExt<I, Output = Result<O>>,
    O: IntoResponse + Send + Sync + 'static,
{
    type Output = Result<Response>;

    async fn call(&self, req: Request) -> Self::Output {
        self.0
            .call(req)
            .await
            .map(IntoResponse::into_response)
            .map_err(IntoResponse::into_error)
    }
}
