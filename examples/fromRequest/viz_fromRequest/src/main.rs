use std::net::SocketAddr;
use tokio::net::TcpListener;
use viz_fromRequest::valid::{ValidJson,ValidQuery};
use serde::{Serialize,Deserialize};
use viz::{serve, IntoHandler, Response, ResponseExt, Result, Router};

#[derive(Debug, Deserialize,Serialize)]
struct UserRequest{
    name: String,
    pwd: String,
}
//ValidJson
async fn user_json_handler(ValidJson(info):ValidJson<UserRequest>) -> Result<Response> {
    Ok(Response::json(format!("user: {}---pwd: {}",info.name,info.pwd))?)
}
//ValidQuery
async fn user_query_handler(ValidQuery(info):ValidQuery<UserRequest>) -> Result<Response> {
    Ok(Response::json(format!("user: {}---pwd: {}",info.name,info.pwd))?)
}
#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("listening on http://{addr}");

    let app = Router::new().get("/",user_json_handler.into_handler()).post("/",user_query_handler.into_handler());

    if let Err(e) = serve(listener, app).await {
        println!("{e}");
    }

    Ok(())
}