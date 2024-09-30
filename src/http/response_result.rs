use std::fmt::{Debug, Display};

use poem::{error::ResponseError, IntoResponse};
use poem_openapi::{
    payload::Json,
    registry::{MetaResponses, Registry},
    types::{ParseFromJSON, ToJSON, Type},
    ApiResponse, Object,
};

#[derive(Debug, Object)]
struct ApiSuccessJson<T: Type + ToJSON + ParseFromJSON> {
    pub status: bool,
    pub data: T,
}

#[derive(Debug, Object)]
struct ApiErrorJson {
    pub status: bool,
    pub error: String,
    pub message: String,
}

pub type ApiRes<P, E> = Result<ApiResPayload<P>, ApiResError<E>>;

pub struct ApiResPayload<P>(pub P);

impl<P: Type + ToJSON + ParseFromJSON> IntoResponse for ApiResPayload<P> {
    fn into_response(self) -> poem::Response {
        Json(ApiSuccessJson { status: true, data: self.0 }).into_response()
    }
}

impl<P> From<P> for ApiResPayload<P> {
    fn from(value: P) -> Self {
        ApiResPayload(value)
    }
}

#[derive(thiserror::Error)]
pub struct ApiResError<E>(pub E);

impl<E> From<E> for ApiResError<E> {
    fn from(value: E) -> Self {
        ApiResError(value)
    }
}

impl<E: Debug + Display> Display for ApiResError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        log::error!("ApiResError: {}", self.0);
        f.write_fmt(format_args!("{{\"status\": false, \"error\": \"{}\"}}", self.0))
    }
}

impl<E: Display + Debug> Debug for ApiResError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        log::error!("ApiResError: {:?}", self.0);
        f.write_fmt(format_args!("{{\"status\": false, \"error\": \"{}\"}}", self.0))
    }
}

impl<E> ResponseError for ApiResError<E> {
    fn as_response(&self) -> poem::Response
    where
        Self: std::error::Error + Send + Sync + 'static,
    {
        let mut resp = self.to_string().into_response();
        resp.set_status(self.status());
        resp
    }

    fn status(&self) -> poem::http::StatusCode {
        poem::http::StatusCode::BAD_REQUEST
    }
}

impl<P: Type + ToJSON + ParseFromJSON> ApiResponse for ApiResPayload<P> {
    const BAD_REQUEST_HANDLER: bool = false;

    fn meta() -> MetaResponses {
        Json::<ApiSuccessJson<P>>::meta()
    }

    fn register(registry: &mut Registry) {
        Json::<ApiSuccessJson<P>>::register(registry);
    }
}

impl<E: Display + Debug> ApiResponse for ApiResError<E> {
    const BAD_REQUEST_HANDLER: bool = false;

    fn meta() -> MetaResponses {
        let mut meta = Json::<ApiErrorJson>::meta();
        meta.responses[0].status = Some(400);
        meta
    }

    fn register(registry: &mut Registry) {
        Json::<ApiErrorJson>::register(registry);
    }
}
