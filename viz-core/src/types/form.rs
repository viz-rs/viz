use std::ops::{Deref, DerefMut};

use bytes::buf::BufExt;
use serde::de::DeserializeOwned;

use viz_utils::{futures::future::BoxFuture, log, serde::urlencoded};

use crate::{
    config::ContextExt as _, get_length, get_mime, Context, Extract, Payload, PayloadCheck,
    PayloadError, PAYLOAD_LIMIT,
};

/// Context Extends
pub trait ContextExt {
    fn form<'a, T>(&'a mut self) -> BoxFuture<'a, Result<T, PayloadError>>
    where
        T: DeserializeOwned + Send + Sync;
}

impl ContextExt for Context {
    fn form<'a, T>(&'a mut self) -> BoxFuture<'a, Result<T, PayloadError>>
    where
        T: DeserializeOwned + Send + Sync,
    {
        Box::pin(async move {
            let mut payload = form::<T>();

            payload.set_limit(self.config().limits.form);

            let m = get_mime(self);
            let l = get_length(self);

            payload.check_header(m, l)?;

            // payload.replace(
            //     urlencoded::from_reader(
            //         payload
            //             .check_real_length(self.take_body().ok_or_else(|| PayloadError::Read)?)
            //             .await?
            //             .reader(),
            //     )
            //     // .map(|o| Form(o))
            //     .map_err(|e| {
            //         log::debug!("{}", e);
            //         PayloadError::Parse
            //     })?,
            // );

            urlencoded::from_reader(
                payload
                    .check_real_length(self.take_body().ok_or_else(|| PayloadError::Read)?)
                    .await?
                    .reader(),
            )
            // .map(|o| Form(o))
            .map_err(|e| {
                log::debug!("{}", e);
                PayloadError::Parse
            })

            // Ok(payload.take())
        })
    }
}

/// Form Extractor
pub struct Form<T>(pub T);

impl<T> Form<T> {
    /// Deconstruct to an inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Form<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> PayloadCheck for Form<T> {
    fn check_type(m: &mime::Mime) -> bool {
        is_form(m)
    }
}

impl<T> Extract for Form<T>
where
    T: DeserializeOwned + Send + Sync,
{
    type Error = PayloadError;

    #[inline]
    fn extract<'a>(cx: &'a mut Context) -> BoxFuture<'a, Result<Self, Self::Error>> {
        Box::pin(async move { cx.form().await.map(|v| Form(v)) })
    }
}

pub fn form<T>() -> Payload<Form<T>>
where
    T: DeserializeOwned,
{
    Payload::new()
}

fn is_form(m: &mime::Mime) -> bool {
    m.type_() == mime::APPLICATION && m.subtype() == mime::WWW_FORM_URLENCODED
}