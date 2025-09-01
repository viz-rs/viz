use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use viz::{
    IntoHandler, Request, RequestExt, Response, ResponseExt, Result, Router, serve,
    types::{Form, Json, Params, Query},
};

#[derive(Debug, Deserialize, Serialize)]

pub struct UserRequest {
    user_name: String,
    pwd: String,
}

//Json
async fn user_login_handler(Json(binding): Json<UserRequest>) -> Result<Response> {
    Ok(Response::json(format!(
        "user_name: {},pwd: {}",
        binding.user_name, binding.pwd
    ))?)
}
//or
//不能直接这样写
//You can't write it directly like this
//(mut req: Request,Json(biding): Json<UserRequest>) //需要手动实现IntoHandler 0.10版本|Need to manually implement IntoHandler 0.10 version
// impl<I, E> IntoHandler<I, E> for UserRequest {}
async fn user_login_or_handler(mut req: Request) -> Result<Response> {
    let Json(binding) = req.extract::<Json<UserRequest>>().await?;
    Ok(Response::json(format!(
        "user_name: {},pwd: {}",
        binding.user_name, binding.pwd
    ))?)
}
//Query
async fn user_info_handler(Query(id): Query<u32>) -> Result<Response> {
    Ok(Response::text(format!("id: {:?}", id)))
}
//or
//不能直接这样写
//You can't write it directly like this
// 同样的(mut req: Request,Query(id): Query<u32>)  //需要手动实现IntoHandler 0.10版本|Need to manually implement IntoHandler 0.10 version
async fn user_info_or_handler(mut req: Request) -> Result<Response> {
    let id = req.extract::<Query<u32>>().await?;
    Ok(Response::text(format!("id= {:?}", id)))
}

//Query
#[derive(Debug, Deserialize)]
struct Pagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    //key
}

//Query
async fn user_list_handler(
    Query(Pagination { offset, limit }): Query<Pagination>,
) -> Result<Response> {
    Ok(Response::text(format!(
        "offset: {:?}, limit: {:?}",
        offset, limit
    )))
}
//Query
//or
async fn user_list_or_handler(mut req: Request) -> Result<Response> {
    let Query(Pagination { offset, limit }) = req.extract::<Query<Pagination>>().await?;
    Ok(Response::text(format!(
        "offset: {:?}, limit: {:?}",
        offset, limit
    )))
}
//Query
//or
async fn user_list_id_handler(mut req: Request) -> Result<Response> {
    let (id, Query(Pagination { offset, limit })) =
        req.extract::<(Params<u32>, Query<Pagination>)>().await?;
    Ok(Response::text(format!(
        "id: {:?} offset: {:?}, limit: {:?}",
        id, offset, limit
    )))
}
//Form
//更新的情况也可以使用Json的方式|update:You can also use Json
async fn user_update_handler(Form(binding): Form<UserRequest>) -> Result<Response> {
    Ok(Response::json(binding)?)
}
//or
//不能直接这样写
//You can't write it directly like this
// 同样的(mut req: Request,Form(info): Form<UserRequest>)  //需要手动实现IntoHandler 0.10版本|Need to manually implement IntoHandler 0.10 version
async fn user_update_or_handler(mut req: Request) -> Result<Response> {
    let info = req.extract::<Form<UserRequest>>().await?.into_inner();
    Ok(Response::json(info)?)
}
//Params
//类似axum和actix中的Path 
//Similar to Path in axum and actix
//("/user/:id")
async fn user_info_path_handler(Params(id): Params<u32>) -> Result<Response> {
    Ok(Response::text(format!("id: {:?}", id)))
}
//or
async fn user_info_path_or_handler(mut req: Request) -> Result<Response> {
    let id = req.extract::<Params<u32>>().await?;
    Ok(Response::text(format!("id: {:?}", id)))
}

//del
//from
async fn user_del_handler(Params(id): Params<u32>) -> Result<Response> {
    Ok(Response::json(format!("id: {:?}", id))?)
}
//删除多个id多数情况使用Json
//Delete multiple ids in most cases using Json
//json
async fn user_del_or_handler(Json(ids): Json<Vec<u32>>) -> Result<Response> {
    Ok(Response::json(format!("ids: {:?}", ids))?)
}
#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("listening on http://{addr}");

    let app = Router::new()
        .post("/user/login", user_login_handler.into_handler())
        //.post("/user/login", user_login_or_handler)
        .get("/user", user_info_handler.into_handler())
        //.get("/user", user_info_or_handler)
        .get("/user_list", user_list_handler.into_handler())
        .get("/user_list_id", user_list_id_handler)
        .put("/user", user_update_handler.into_handler())
       // .put("/user", user_update_or_handler)
         .get("/user/:id", user_info_path_handler.into_handler())
         //.get("/user/:id", user_info_path_or_handler)
        .delete("/user/:id", user_del_handler.into_handler())
        //.delete("/user", user_del_or_handler.into_handler())
        ;

    if let Err(e) = serve(listener, app).await {
        println!("{e}");
    }

    Ok(())
}
