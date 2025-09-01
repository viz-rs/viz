use crate::api_err::ApiError;
use serde::Serialize;
use viz::{FromRequest, Request, RequestExt};

#[derive(Serialize, Debug)]
pub struct ValidJson<T>(pub T);
impl<T> FromRequest for ValidJson<T>
where
    T: serde::de::DeserializeOwned,
{
    type Error = ApiError;
    async fn extract(req: &mut Request) -> Result<Self, Self::Error> {
        match req.json().await {
            Ok(data) => Ok(ValidJson(data)),
            Err(e) => return Err(ApiError::from(e)),//api_err已经处理PayloadError的错误
        }
        //req.json().await.map_err(ApiError::from)?;
        // Ok(req.json().await.map(Self)?)
    }
}
#[derive(Serialize, Debug)]
pub struct ValidQuery<T>(pub T);

impl<T> FromRequest for ValidQuery<T>
where
    T: serde::de::DeserializeOwned + Send,
{
    type Error = ApiError;

    async fn extract(req: &mut Request) -> Result<Self, Self::Error> {
        match req.query(){
            Ok(data)=> Ok(ValidQuery(data)),
            Err(e) => Err(ApiError::from(e)),//api_err已经处理PayloadError的错误
        }
        // let inner:T = req.query().map_err(|e| ApiError::from(e))?;
    }
}
