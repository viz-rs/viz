use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use bytes::buf::BufExt;
use serde::{de::DeserializeOwned, Serialize, Serializer};

use viz_utils::{futures::future::BoxFuture, log, serde::json};

use crate::{
    config::ContextExt as _, get_length, get_mime, Context, Extract, Payload, PayloadCheck,
    PayloadError,
};

/// Context Extends
pub trait ContextExt {
    fn json<'a, T>(&'a mut self) -> BoxFuture<'a, Result<T, PayloadError>>
    where
        T: DeserializeOwned + Send + Sync;
}

impl ContextExt for Context {
    fn json<'a, T>(&'a mut self) -> BoxFuture<'a, Result<T, PayloadError>>
    where
        T: DeserializeOwned + Send + Sync,
    {
        Box::pin(async move {
            let mut payload = json::<T>();

            payload.set_limit(self.config().limits.json);

            let m = get_mime(self);
            let l = get_length(self);

            payload.check_header(m, l)?;

            json::from_reader(
                payload
                    .check_real_length(self.take_body().ok_or_else(|| PayloadError::Read)?)
                    .await?
                    .reader(),
            )
            .map_err(|e| {
                log::debug!("{}", e);
                PayloadError::Parse
            })
        })
    }
}

/// Json Extractor
pub struct Json<T>(pub T);

impl<T> Serialize for Json<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        T::serialize(self, serializer)
    }
}

impl<T> Json<T> {
    /// Deconstruct to an inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> AsRef<T> for Json<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> PayloadCheck for Json<T> {
    fn check_type(m: &mime::Mime) -> bool {
        is_json(m)
    }
}

impl<T: fmt::Debug> fmt::Debug for Json<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::fmt(&self, f)
    }
}

impl<T> Extract for Json<T>
where
    T: DeserializeOwned + Send + Sync,
{
    type Error = PayloadError;

    #[inline]
    fn extract<'a>(cx: &'a mut Context) -> BoxFuture<'a, Result<Self, Self::Error>> {
        Box::pin(async move { cx.json().await.map(|v| Json(v)) })
    }
}

/// Creates a JSON payload
pub fn json<T>() -> Payload<Json<T>>
where
    T: DeserializeOwned,
{
    Payload::new()
}

fn is_json(m: &mime::Mime) -> bool {
    m.type_() == mime::APPLICATION && (m.subtype() == mime::JSON || m.suffix() == Some(mime::JSON))
}
