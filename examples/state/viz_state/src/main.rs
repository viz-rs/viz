use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use viz::types::State;
use viz::{ Handler, Next, Request, RequestExt, Response, Result, Router, async_trait, serve };

//handle
async fn index(_: Request) -> Result<&'static str> {
    Ok("Hello, Viz!")
}
async fn index_db(req: Request) -> Result<String> {
    let state = req.state::<AppState>().unwrap();
    Ok(format!("default_state: {}", state.db))
}
async fn index_middleware_db(req: Request) -> Result<String> {
    //多数情况使用它
    //Use it in most cases
    let state = req.extensions().get::<AppState>().unwrap();
    //当你需要用State包裹
    //If you need to wrap it with State
    //let state = State(req.extensions().get::<AppState>().unwrap());
    Ok(format!("middleware_state: {}", state.db))
}
async fn index_middleware_struct_db(req: Request) -> Result<String> {
    //多数情况使用它
    //Use it in most cases
    let state = req.extensions().get::<AppState>().unwrap();
    //当你需要用State包裹
    //If you need to wrap it with State
    //let state = State(req.extensions().get::<AppState>().unwrap());
    Ok(format!("middleware_struct_state: {}", state.db))
}
//state 定义
//State Definition
#[derive(Debug, Clone)]
pub struct AppState {
    pub db: Arc<String>,
}
//state new
impl AppState {
    pub fn new(db: Arc<String>) -> Self {
        Self { db }
    }
}
//middleware
//简单的中间件实现共享State
//Simple middleware to implement shared State
async fn middleware_state<H>((mut req, handler): Next<Request, H>) -> Result<Response>
    where H: Handler<Request, Output = Result<Response>>
{
    let state = AppState {
        db: Arc::new(String::from("middleware_mysql")),
    };
    req.extensions_mut().insert(state);
    // before ...
    handler.call(req).await
    // after ...
}
// middleware struct
//简单Struct自定义的中间件实现共享State
//Simple middleware struct to implement shared State
#[derive(Clone)]
struct StateMiddlewareStruct {}

#[async_trait]
impl<H> Handler<Next<Request, H>> for StateMiddlewareStruct where H: Handler<Request> {
    type Output = H::Output;

    async fn call(&self, (mut r, h): Next<Request, H>) -> Self::Output {
        let sate = AppState {
            db: Arc::new(String::from("struct Middleware Mysql")),
        };
        r.extensions_mut().insert(sate);
        h.call(r).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    //State的实例化
    //State instantiation
    let state = AppState {
        db: Arc::new(String::from("default_mysql")),
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("listening on http://{addr}");
    let app = Router::new()
        .get("/", index) //hello word
        .get("/db", index_db)
        .with(State::new(state)) //使用viz::types::State共享//Using viz::types::State sharing
        .get("/middleware/db", index_middleware_db)
        .with_handler(middleware_state) //使用简单中间件State共享
        .get("/middleware/struct_db", index_middleware_struct_db)
        .with_handler(StateMiddlewareStruct {}); //使用简单的struct中间件State共享//Use simple struct middleware State sharing

    if let Err(e) = serve(listener, app).await {
        println!("{e}");
    }

    Ok(())
}
