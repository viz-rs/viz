use core::fmt;

use viz_core::{
    handler::{Next, Transform},
    Body, BoxHandler, FnExt, FromRequest, Handler, HandlerExt, IntoResponse, Method, Request,
    Responder, ResponderExt, Response, Result,
};

macro_rules! repeat {
    ($macro:ident $($name:ident $verb:tt )+) => {
        $(
            $macro!($name $verb);
        )+
    };
}

macro_rules! export_internal_verb {
    ($name:ident $verb:tt) => {
        #[doc = concat!(" Appends a route, handle HTTP verb `", stringify!($verb), "`.")]
        pub fn $name<H, O>(self, handler: H) -> Self
        where
            H: Handler<Request<Body>, Output = Result<O>> + Clone,
            O: IntoResponse + Send + Sync + 'static,
        {
            self.on(Method::$verb, handler)
        }
    };
}

#[cfg(feature = "ext")]
macro_rules! export_internal_verb_ext {
    ($name:ident $verb:tt) => {
        #[doc = concat!(" Appends a route, handle HTTP verb `", stringify!($verb), "` with multiple parameters.")]
        pub fn $name<H, O, I>(self, handler: H) -> Self
        where
            I: FromRequest + Send + Sync + 'static,
            I::Error: IntoResponse + Send + Sync,
            H: FnExt<I, Output = Result<O>>,
            O: IntoResponse + Send + Sync + 'static,
        {
            self.on_ext(Method::$verb, handler)
        }
    };
}

macro_rules! export_verb {
    ($name:ident $verb:ty) => {
        #[doc = concat!(" Appends a route, handle HTTP verb `", stringify!($verb), "`.")]
        pub fn $name<H, O>(handler: H) -> Route
        where
            H: Handler<Request<Body>, Output = Result<O>> + Clone,
            O: IntoResponse + Send + Sync + 'static,
        {
            Route::new().$name(handler)
        }
    };
}

#[cfg(feature = "ext")]
macro_rules! export_verb_ext {
    ($name:ident $verb:ty) => {
        #[doc = concat!(" Appends a route, handle HTTP verb `", stringify!($verb), "` with multiple parameters.")]
        pub fn $name<H, O, I>(handler: H) -> Route
        where
            I: FromRequest + Send + Sync + 'static,
            I::Error: IntoResponse + Send + Sync,
            H: FnExt<I, Output = Result<O>>,
            O: IntoResponse + Send + Sync + 'static,
        {
            Route::new().$name(handler)
        }
    };
}

#[derive(Clone)]
pub struct Route {
    pub(crate) methods: Vec<(Method, BoxHandler)>,
}

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Route")
            .field(
                "methods",
                &self
                    .methods
                    .iter()
                    .map(|(m, _)| m)
                    .collect::<Vec<&Method>>(),
            )
            .finish()
    }
}

impl Route {
    pub fn new() -> Self {
        Self {
            methods: Vec::new(),
        }
    }

    pub fn push(mut self, method: Method, handler: BoxHandler) -> Self {
        match self
            .methods
            .iter_mut()
            .find(|(m, _)| m == method)
            .map(|(_, e)| e)
        {
            Some(h) => *h = handler,
            None => self.methods.push((method, handler)),
        }

        self
    }

    /// Appends a route, with a HTTP verb and handler.
    pub fn on<H, O>(self, method: Method, handler: H) -> Self
    where
        H: Handler<Request<Body>, Output = Result<O>> + Clone,
        O: IntoResponse + Send + Sync + 'static,
    {
        self.push(method, Responder::new(handler).boxed())
    }

    /// Appends a route, with a HTTP verb and handler.
    pub fn any<H, O>(self, handler: H) -> Self
    where
        H: Handler<Request<Body>, Output = Result<O>> + Clone,
        O: IntoResponse + Send + Sync + 'static,
    {
        [
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::HEAD,
            Method::OPTIONS,
            Method::CONNECT,
            Method::PATCH,
            Method::TRACE,
        ]
        .into_iter()
        .fold(self, |route, method| route.on(method, handler.clone()))
    }

    repeat!(
        export_internal_verb
        get GET
        post POST
        put PUT
        delete DELETE
        head HEAD
        options OPTIONS
        connect CONNECT
        patch PATCH
        trace TRACE
    );

    pub fn with<T>(self, t: T) -> Self
    where
        T: Transform<BoxHandler>,
        T::Output: Handler<Request<Body>, Output = Result<Response<Body>>>,
    {
        self.into_iter()
            .map(|(method, handler)| (method, t.transform(handler).boxed()))
            .collect()
    }

    pub fn with_handler<F>(self, f: F) -> Self
    where
        F: Handler<Next<Request<Body>, BoxHandler>, Output = Result<Response<Body>>> + Clone,
    {
        self.into_iter()
            .map(|(method, handler)| (method, handler.around(f.clone()).boxed()))
            .collect()
    }

    pub fn map_handler<F>(self, f: F) -> Self
    where
        F: Fn(BoxHandler) -> BoxHandler,
    {
        self.into_iter()
            .map(|(method, handler)| (method, f(handler)))
            .collect()
    }
}

#[cfg(feature = "ext")]
impl Route {
    /// Appends a route, with a HTTP verb and handler.
    pub fn on_ext<H, O, I>(self, method: Method, handler: H) -> Self
    where
        I: FromRequest + Send + Sync + 'static,
        I::Error: IntoResponse + Send + Sync,
        H: FnExt<I, Output = Result<O>>,
        O: IntoResponse + Send + Sync + 'static,
    {
        self.push(method, ResponderExt::new(handler).boxed())
    }

    /// Appends a route, with a HTTP verb and handler.
    pub fn any_ext<H, O, I>(self, handler: H) -> Self
    where
        I: FromRequest + Send + Sync + 'static,
        I::Error: IntoResponse + Send + Sync,
        H: FnExt<I, Output = Result<O>>,
        O: IntoResponse + Send + Sync + 'static,
    {
        [
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::HEAD,
            Method::OPTIONS,
            Method::CONNECT,
            Method::PATCH,
            Method::TRACE,
        ]
        .into_iter()
        .fold(self, |route, method| route.on_ext(method, handler.clone()))
    }

    repeat!(
        export_internal_verb_ext
        get_ext GET
        post_ext POST
        put_ext PUT
        delete_ext DELETE
        head_ext HEAD
        options_ext OPTIONS
        connect_ext CONNECT
        patch_ext PATCH
        trace_ext TRACE
    );
}

impl IntoIterator for Route {
    type Item = (Method, BoxHandler);

    type IntoIter = std::vec::IntoIter<(Method, BoxHandler)>;

    fn into_iter(self) -> Self::IntoIter {
        self.methods.into_iter()
    }
}

impl FromIterator<(Method, BoxHandler)> for Route {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (Method, BoxHandler)>,
    {
        Self {
            methods: iter.into_iter().collect(),
        }
    }
}

/// Appends a route, with a HTTP verb and handler.
pub fn on<H, O>(method: Method, handler: H) -> Route
where
    H: Handler<Request<Body>, Output = Result<O>> + Clone,
    O: IntoResponse + Send + Sync + 'static,
{
    Route::new().on(method, handler)
}

repeat!(
    export_verb
    get GET
    post POST
    put PUT
    delete DELETE
    head HEAD
    options OPTIONS
    connect CONNECT
    patch PATCH
    trace TRACE
);

#[cfg(feature = "ext")]
/// Appends a route, with a HTTP verb and multiple parameters of handler.
pub fn on_ext<H, O, I>(method: Method, handler: H) -> Route
where
    I: FromRequest + Send + Sync + 'static,
    I::Error: IntoResponse + Send + Sync,
    H: FnExt<I, Output = Result<O>>,
    O: IntoResponse + Send + Sync + 'static,
{
    Route::new().on_ext(method, handler)
}

#[cfg(feature = "ext")]
repeat!(
    export_verb_ext
    get_ext GET
    post_ext POST
    put_ext PUT
    delete_ext DELETE
    head_ext HEAD
    options_ext OPTIONS
    connect_ext CONNECT
    patch_ext PATCH
    trace_ext TRACE
);

#[cfg(test)]
mod tests {
    use super::Route;
    use std::sync::Arc;
    use viz_core::{
        async_trait,
        handler::Transform,
        types::{self, Data, Query},
        Body, Handler, HandlerExt, IntoResponse, Method, Next, Request, Response, Result,
    };

    #[tokio::test]
    async fn route() -> anyhow::Result<()> {
        async fn handler(_: Request<Body>) -> Result<impl IntoResponse> {
            Ok(())
        }

        struct Logger;

        impl Logger {
            fn new() -> Self {
                Self
            }
        }

        impl<H: Clone> Transform<H> for Logger {
            type Output = LoggerHandler<H>;

            fn transform(&self, h: H) -> Self::Output {
                LoggerHandler(h.clone())
            }
        }

        #[derive(Clone)]
        struct LoggerHandler<H>(H);

        #[async_trait]
        impl<H> Handler<Request<Body>> for LoggerHandler<H>
        where
            H: Handler<Request<Body>> + Clone,
        {
            type Output = H::Output;

            async fn call(&self, req: Request<Body>) -> Self::Output {
                dbg!("before logger");
                let res = self.0.call(req).await;
                dbg!("after logger");
                res
            }
        }

        async fn before(req: Request<Body>) -> Result<Request<Body>> {
            dbg!("before req");
            Ok(req)
        }

        async fn after(res: Result<Response<Body>>) -> Result<Response<Body>> {
            dbg!("after res");
            res
        }

        async fn around<H, O>((req, handler): Next<Request<Body>, H>) -> Result<Response<Body>>
        where
            H: Handler<Request<Body>, Output = Result<O>> + Clone,
            O: IntoResponse + Send + Sync + 'static,
        {
            dbg!("around before");
            let res = handler.call(req).await.map(IntoResponse::into_response);
            dbg!("around after");
            res
        }

        async fn around_1<H, O>((req, handler): Next<Request<Body>, H>) -> Result<Response<Body>>
        where
            H: Handler<Request<Body>, Output = Result<O>> + Clone,
            O: IntoResponse + Send + Sync + 'static,
        {
            dbg!("around before --- 1");
            let res = handler.call(req).await.map(IntoResponse::into_response);
            dbg!("around after  --- 1");
            res
        }

        async fn around_2<H>((req, handler): Next<Request<Body>, H>) -> Result<Response<Body>>
        where
            H: Handler<Request<Body>, Output = Result<Response<Body>>> + Clone,
        {
            dbg!("around before ---> 2");
            let res = handler.call(req).await;
            dbg!("around after  <--- 2");
            res
        }

        #[derive(Clone)]
        struct Around2 {
            name: String,
        }

        #[async_trait]
        impl<H, I, O> Handler<Next<I, H>> for Around2
        where
            I: Send + 'static,
            H: Handler<I, Output = Result<O>> + Clone,
        {
            type Output = H::Output;

            async fn call(&self, (i, h): Next<I, H>) -> Self::Output {
                dbg!(format!("around before --- {}", &self.name));
                let res = h.call(i).await;
                dbg!(format!("around after  --- {}", &self.name));
                res
            }
        }

        #[derive(Clone)]
        struct Around3 {
            name: String,
        }

        #[async_trait]
        impl<H, O> Handler<Next<Request<Body>, H>> for Around3
        where
            H: Handler<Request<Body>, Output = Result<O>> + Clone,
            O: IntoResponse,
        {
            type Output = Result<Response<Body>>;

            async fn call(&self, (i, h): Next<Request<Body>, H>) -> Self::Output {
                dbg!(format!("around before --- {}", &self.name));
                let res = h.call(i).await.map(IntoResponse::into_response);
                dbg!(format!("around after  --- {}", &self.name));
                res
            }
        }

        #[derive(Clone)]
        struct Around4 {
            name: String,
        }

        #[async_trait]
        impl<H> Handler<Next<Request<Body>, H>> for Around4
        where
            H: Handler<Request<Body>, Output = Result<Response<Body>>> + Clone,
        {
            type Output = Result<Response<Body>>;

            async fn call(&self, (i, h): Next<Request<Body>, H>) -> Self::Output {
                dbg!(format!("around before ---> {}", &self.name));
                let res = h.call(i).await;
                dbg!(format!("around after  <--- {}", &self.name));
                res
            }
        }

        async fn ext(q: Query<usize>, d: Data<Arc<String>>) -> Result<impl IntoResponse> {
            dbg!(377);
            Ok(vec![233])
        }

        let route = Route::new()
            .any_ext(ext)
            .on(Method::GET, handler.before(before))
            .on(Method::POST, handler.after(after))
            .put(handler.around(Around2 {
                name: "handler around".to_string(),
            }))
            .with(Logger::new())
            .map_handler(|handler| {
                handler
                    .before(before)
                    .around(Around4 {
                        name: "4".to_string(),
                    })
                    .after(after)
                    .around(around_2)
                    .around(Around2 {
                        name: "2".to_string(),
                    })
                    .around(around)
                    .around(around_1)
                    .around(Around3 {
                        name: "3".to_string(),
                    })
                    .with(Logger::new())
                    .boxed()
            })
            .with_handler(around)
            .with_handler(around_1)
            .with_handler(around_2)
            .with_handler(Around2 {
                name: "2 with handler".to_string(),
            })
            .with_handler(Around3 {
                name: "3 with handler".to_string(),
            })
            .with_handler(Around4 {
                name: "4 with handler".to_string(),
            })
            // .with(viz_core::middleware::cookie::Config::new())
            .into_iter()
            .map(|(method, handler)| (method, handler))
            // .filter(|(method, _)| method != Method::GET)
            .collect::<Route>();

        dbg!(std::mem::size_of_val(&route));

        let (_, h) = route
            .methods
            .iter()
            .filter(|(m, _)| m == Method::GET)
            .nth(0)
            .unwrap();

        let res = match h.call(Request::default()).await {
            Ok(r) => r,
            Err(e) => e.into_response(),
        };

        dbg!(res);

        Ok(())
    }
}
