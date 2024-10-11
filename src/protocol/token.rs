use poem_openapi::Object;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TokenApiError {
    #[error("WrongSecret")]
    WrongSecret,
}

#[derive(Debug, Object)]
pub struct CreateNotifyTokenRequest {
    pub client_id: String,
    pub ttl: u64,
}

#[derive(Debug, Object)]
pub struct CreateNotifyTokenResponse {
    pub token: String,
}
