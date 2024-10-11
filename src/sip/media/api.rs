use std::time::Duration;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaApiError {
    #[error("HttpError ({0})")]
    Http(#[from] reqwest::Error),
    #[error("MediaError ({0})")]
    Media(String),
}

pub type Result<T> = std::result::Result<T, MediaApiError>;

#[derive(Deserialize)]
struct TokenData {
    token: String,
}

#[derive(Deserialize)]
struct CreateTokenResponse {
    // status: bool,
    error: Option<String>,
    data: Option<TokenData>,
}

#[derive(Debug, Clone)]
pub struct MediaApi {
    gateway: String,
    secret: String,
}

impl MediaApi {
    pub fn new(gateway: &str, secret: &str) -> Self {
        Self {
            gateway: gateway.to_string(),
            secret: secret.to_string(),
        }
    }

    pub fn gateway(&self) -> &str {
        &self.gateway
    }

    pub async fn create_rtpengine_token(&self, room: &str, peer: &str, record: bool) -> Result<String> {
        let res: CreateTokenResponse = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Should create client")
            .post(&format!("{}/token/rtpengine", self.gateway))
            .header("Authorization", format!("Bearer {}", self.secret))
            .json(&serde_json::json!({
                "room": room,
                "peer": peer,
                "ttl": 3600,
                "record": record
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if let Some(data) = res.data {
            Ok(data.token)
        } else {
            Err(MediaApiError::Media(res.error.unwrap_or_else(|| "Unknown".to_string())))
        }
    }

    #[allow(unused)]
    pub async fn create_webrtc_token(&self, room: &str, peer: &str, record: bool) -> Result<String> {
        let res: CreateTokenResponse = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Should create client")
            .post(&format!("{}/token/webrtc", self.gateway))
            .header("Authorization", format!("Bearer {}", self.secret))
            .json(&serde_json::json!({
                "room": room,
                "peer": peer,
                "ttl": 3600,
                "record": record
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if let Some(data) = res.data {
            Ok(data.token)
        } else {
            Err(MediaApiError::Media(res.error.unwrap_or_else(|| "Unknown".to_string())))
        }
    }
}
