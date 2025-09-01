use thiserror::Error;
use viz::{types::Json, IntoResponse, Response, StatusCode};
use viz::types::{ PayloadError};

//必须处理 PayloadError的from|TryFrom转换
//Must handle PayloadError's from|TryFrom conversion
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("CommonParse Err Is:{0}")] CommonParse(PayloadError),
    #[error("VizInternalError Is:{0}")]
    VizInternalError(viz::Error),

}

//必须实现 IntoResponse trait
//Must implement the IntoResponse trait
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json::new(self.to_string()).into_response().into_body();
        Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(body).unwrap()
    }
}
//PayloadError
impl From<PayloadError> for ApiError {
    fn from(e: PayloadError) -> Self {
        Self::CommonParse(e)
    }
}
//viz::Error
impl From<viz::Error> for ApiError {
    fn from(e: viz::Error) -> Self {
        Self::VizInternalError(e)
    }
}