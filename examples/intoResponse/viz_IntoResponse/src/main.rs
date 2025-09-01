use serde::Serialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use viz::{
    Body, Error, IntoResponse, Request, Response, ResponseExt, Result, Router, StatusCode,
    serve,
};

//响应的数据定义
//Response data definition
#[derive(Serialize, Debug)]
struct ApiResponse<T> {
    code: u16,
    data: T,
}
impl<T> ApiResponse<T> {
    fn new(code: u16, data: T) -> Self {
        Self { code, data }
    }
}
//success
impl<T> ApiResponse<T> {
    pub fn success(code: u16, data: T) -> Self {
        Self::new(code, data)
    }
}
//fail
impl<T> ApiResponse<T> {
    pub fn fail_with_data(data: T) -> Self {
        Self::new(400, data)
    }
}
//IntoRespons trait 的实现
//Implementation of the IntoRespons trait
impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let body = Response::json(&self).into_response().into_body();
        Response::builder()
            .status(StatusCode::from_u16(self.code).unwrap())
            .body(body)
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::default())
                    .unwrap()
            }) // 提供错误处理//Provide error handling
    }
    //多数情况下使用默认实现
    //Use the default implementation in most cases
    fn into_error(self) -> Error {
        Error::Responder(self.into_response()) //default
    }
}
//Err
//自定义错误枚举
//Error Enum
#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error("Not Found")]
    NotFound,
    #[error("FileLoad Error:{0}")]
    FileLoad(#[from] std::io::Error),
}
impl MyError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            MyError::NotFound => StatusCode::NOT_FOUND,
            MyError::FileLoad(_) => StatusCode::INSUFFICIENT_STORAGE,
        }
    }
}

//类型转换
//将MyError类型自动转换为自定义的Error类型
//Automatically convert the MyError type to a custom Error type
impl From<MyError> for Error {
    fn from(e: MyError) -> Self {
        match e {
            MyError::NotFound => Error::Report(Box::new(e), MyError::NotFound.into_response()),
            MyError::FileLoad(_) => {
                Error::Report(Box::new(e), std::io::Error::last_os_error().into_response())
            }
        }
    }
}

//自定义错误的IntoResponse
//Custom Error IntoResponse
impl IntoResponse for MyError {
    fn into_response(self) -> Response {
        let body = ApiResponse::fail_with_data(()).into_response().into_body();
        Response::builder()
            .status(self.status_code())
            .extension(self.to_string())
            .body(body)
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap()
            }) // 提供构建错误处理//Provide error handling
    }
    //多数情况下使用默认实现
    //Use the default implementation in most cases
    //default
    fn into_error(self) -> Error {
        //Error::Responder(self.into_response())
        Error::Boxed(Box::new(self))
    }
}
//handle
async fn index_handle(_: Request) -> Result<&'static str> {
    Ok("Hello, Viz!")
}
async fn index_api_handle(_: Request) -> Result<ApiResponse<&'static str>> {
    let data = "hello this is a data response";
    Ok(ApiResponse::success(200, data))
}
async fn index_err_handle(_: Request) -> Result<ApiResponse<()>> {
    let data = ApiResponse::fail_with_data(());
    Ok(data)
}
//or
async fn index_enum_err_handle(_: Request) -> Result<ApiResponse<()>> {
    Err(MyError::NotFound.into())
}
#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("listening on http://{addr}");

    let app = Router::new()
        .get("/", index_handle)
        .get("api", index_api_handle)
        .get("/err", index_err_handle)
        .get("/enum_err", index_enum_err_handle);
    if let Err(e) = serve(listener, app).await {
        println!("{e}");
    }

    Ok(())
}
