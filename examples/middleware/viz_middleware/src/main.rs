use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpListener;
use tokio::time::timeout;
use viz::{
    Body, Handler, HandlerExt, IntoHandler, IntoResponse, Next, Request, Response, ResponseExt,
    Result, Router, StatusCode, Transform, async_trait, get, serve, types::Params,
};

async fn index_handle(_: Request) -> Result<Response> {
    Ok(StatusCode::OK.into_response())
}

async fn not_found_handle(_: Request) -> Result<impl IntoResponse> {
    Ok(StatusCode::OK)
}

async fn show_user_handle(Params(id): Params<u64>) -> Result<impl IntoResponse> {
    Ok(format!("post {}", id))
}

// middleware fn
async fn my_middleware<H>((req, handler): Next<Request, H>) -> Result<Response>
where
    H: Handler<Request, Output = Result<Response>>,
{
    // before ...
    //todo
    // 前操作：记录请求开始
    println!("Request started");
    let result = handler.call(req).await;
    // after ...
    //todo
    // 后操作：处理结果
    match result {
        Ok(res) => Ok(res),
        Err(e) => Err(e),
    }
    //or return result
    //result
}
// middleware struct
#[derive(Clone)]
struct MyMiddleware {}
#[async_trait]
impl<H> Handler<Next<Request, H>> for MyMiddleware
where
    H: Handler<Request, Output = Result<Response>>, //default H:Handler<Request>
{
    type Output = Result<Response>;

    async fn call(&self, (r, h): Next<Request, H>) -> Self::Output {
        match h.call(r).await {
            Ok(result) => Ok(result),
            Err(e) => Err(e),
        }
        // or
        //h.call(r).await
    }
}
// A configuration for Timeout Middleware
struct Timeout {
    delay: Duration,
}

impl Timeout {
    pub fn new(secs: u64) -> Self {
        Self {
            delay: Duration::from_secs(secs),
        }
    }
}

impl<H: Clone> Transform<H> for Timeout {
    type Output = TimeoutMiddleware<H>;

    fn transform(&self, h: H) -> Self::Output {
        TimeoutMiddleware(h, self.delay)
    }
}
// Timeout Middleware
#[derive(Clone)]
struct TimeoutMiddleware<H>(H, Duration);

#[async_trait]
impl<H> Handler<Request> for TimeoutMiddleware<H>
where
    H: Handler<Request>,
{
    type Output = H::Output;

    async fn call(&self, req: Request) -> Self::Output {
        let duration = self.1.clone();
        println!("duration:{:?}", duration);
        //todo
        //优化点使用 match 捕获超时的错误进行处理
        //Handle timeout errors using match
        self.0.call(req).await
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("listening on http://{addr}");

    let app = Router::new()
        .get(
            "/",
            index_handle
                //使用函数中间件
                //Use function middleware
                .around(my_middleware)
                //使用结构体中间件
                //Use struct middleware
                .around(MyMiddleware {})
                //超时中间件
                //Timeout Middleware
                .with(Timeout::new(0)),
        )
        .route(
            "/users/:id",
            get(show_user_handle
                .into_handler() //参考绑定参数 //Converts self to a Handler
                .map_into_response() //Maps the handler’s output type to the Response
                // handler level
                //使用函数中间件
                //Use function middleware
                .around(my_middleware)
                //超时中间件
                //Timeout Middleware
                .with(Timeout::new(0)))
            .post(
                (|_| async { Ok(Response::text("update")) }) //闭包函数的handle//The handle of a closure function
                    // handler level
                    //使用函数中间件
                    //Use function middleware
                    .around(my_middleware)
                    //超时中间件
                    //Timeout Middleware
                    .with(Timeout::new(0)),
            )
            // route level
            //使用结构体中间件
            // Use struct middleware
            .with_handler(MyMiddleware {})
                //超时中间件
                //Timeout Middleware
            .with(Timeout::new(2)),
        )
        .get(
            "/*",
            not_found_handle
                .map_into_response()
                // handler level
                //使用函数体中间件
                .around(my_middleware)
                //使用结构体中间件
                .around(MyMiddleware {}),
        )
        // router level
        //使用函数体中间件
        .with_handler(my_middleware)
        //使用结构体中间件
        .with_handler(MyMiddleware {})
        .with(Timeout::new(4));

    if let Err(e) = serve(listener, app).await {
        println!("{e}");
    }

    Ok(())
}
