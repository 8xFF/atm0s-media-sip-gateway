use thiserror::Error;

mod api;
mod rtp_answer;
mod rtp_offer;

pub use api::*;
pub use rtp_answer::*;
pub use rtp_offer::*;

#[derive(Debug, Error)]
pub enum MediaEngineError {
    #[error("Media error {0}")]
    Media(#[from] MediaApiError),
    #[error("Requwest error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Missing location header")]
    MissingLocation,
    #[error("Invalid localtion value")]
    InvalidLocation,
    #[error("Invalid status code ({0})")]
    InvalidStatus(u16),
    #[error("Invalid body")]
    InvalidBody,
}
