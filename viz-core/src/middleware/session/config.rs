use std::{
    fmt,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use crate::{
    async_trait,
    middleware::helper::{CookieOptions, Cookieable},
    types::Session,
    Error, Handler, IntoResponse, Request, RequestExt, Response, Result, StatusCode, Transform,
};

use super::{Error as SessionError, Storage, Store, PURGED, RENEWED, UNCHANGED};

/// A configuration for [`SessionMiddleware`].
pub struct Config<S, G, V>(Arc<(Store<S, G, V>, CookieOptions)>);

impl<S, G, V> Config<S, G, V> {
    /// Creates a new configuration with the [`Store`] and [`CookieOptions`].
    pub fn new(store: Store<S, G, V>, cookie: CookieOptions) -> Self {
        Self(Arc::new((store, cookie)))
    }

    /// Gets the store.
    #[must_use]
    pub fn store(&self) -> &Store<S, G, V> {
        &self.0 .0
    }

    /// Gets the TTL.
    #[must_use]
    pub fn ttl(&self) -> Option<Duration> {
        self.options().max_age
    }
}

impl<S, G, V> Clone for Config<S, G, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S, G, V> Cookieable for Config<S, G, V> {
    fn options(&self) -> &CookieOptions {
        &self.0 .1
    }
}

impl<S, G, V> fmt::Debug for Config<S, G, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionConfig").finish()
    }
}

impl<H, S, G, V> Transform<H> for Config<S, G, V> {
    type Output = SessionMiddleware<H, S, G, V>;

    fn transform(&self, h: H) -> Self::Output {
        SessionMiddleware {
            h,
            config: self.clone(),
        }
    }
}

/// Session middleware.
#[derive(Debug)]
pub struct SessionMiddleware<H, S, G, V> {
    h: H,
    config: Config<S, G, V>,
}

impl<H, S, G, V> Clone for SessionMiddleware<H, S, G, V>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            h: self.h.clone(),
            config: self.config.clone(),
        }
    }
}

#[async_trait]
impl<H, O, S, G, V> Handler<Request> for SessionMiddleware<H, S, G, V>
where
    O: IntoResponse,
    H: Handler<Request, Output = Result<O>> + Clone,
    S: Storage + 'static,
    G: Fn() -> String + Send + Sync + 'static,
    V: Fn(&str) -> bool + Send + Sync + 'static,
{
    type Output = Result<Response>;

    async fn call(&self, mut req: Request) -> Self::Output {
        let cookies = req.cookies().map_err(Into::<Error>::into)?;
        let cookie = self.config.get_cookie(&cookies);

        let mut session_id = cookie.map(|cookie| cookie.value().to_string());
        let data = match &session_id {
            Some(sid) if (self.config.store().verify)(sid) => self.config.store().get(sid).await?,
            _ => None,
        };
        if data.is_none() && session_id.is_some() {
            session_id.take();
        }
        let session = Session::new(data.unwrap_or_default());
        req.extensions_mut().insert(session.clone());

        let resp = self.h.call(req).await.map(IntoResponse::into_response);

        let status = session.status().load(Ordering::Acquire);

        if status == UNCHANGED {
            return resp;
        }

        if status == PURGED {
            if let Some(sid) = &session_id {
                self.config
                    .store()
                    .remove(sid)
                    .await
                    .map_err(Into::<Error>::into)?;
                self.config.remove_cookie(&cookies);
            }

            return resp;
        }

        if status == RENEWED {
            if let Some(sid) = &session_id.take() {
                self.config
                    .store()
                    .remove(sid)
                    .await
                    .map_err(Into::<Error>::into)?;
            }
        }

        let sid = if let Some(sid) = session_id {
            sid
        } else {
            let sid = (self.config.store().generate)();
            self.config.set_cookie(&cookies, &sid);
            sid
        };

        self.config
            .store()
            .set(
                &sid,
                session.data()?,
                &self.config.ttl().unwrap_or_else(max_age),
            )
            .await
            .map_err(Into::<Error>::into)?;

        resp
    }
}

fn max_age() -> Duration {
    Duration::from_secs(CookieOptions::MAX_AGE)
}

impl From<SessionError> for Error {
    fn from(e: SessionError) -> Self {
        Self::Responder(e.into_response())
    }
}

impl IntoResponse for SessionError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
