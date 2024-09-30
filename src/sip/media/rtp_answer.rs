use std::time::Duration;

use bytes::Bytes;

use crate::protocol::StreamingInfo;

use super::{MediaApi, MediaEngineError};

pub struct MediaRtpEngineAnswer {
    api: MediaApi,
    offer: Bytes,
    created: Option<(String, Bytes)>,
}

impl MediaRtpEngineAnswer {
    pub fn new(api: MediaApi, offer: Bytes) -> Self {
        Self { api, offer, created: None }
    }

    pub async fn create_answer(&mut self, stream: &StreamingInfo) -> Result<Bytes, MediaEngineError> {
        assert!(self.created.is_none(), "should not call create_answer twice");
        log::info!("[MediaRtpEngineAnswer] creating token");
        let token = self.api.create_rtpengine_token(&stream.room, &stream.peer, stream.record).await?;
        log::info!("[MediaRtpEngineAnswer] created token");
        log::info!("[MediaRtpEngineAnswer] creating answer");
        let res = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Should create client")
            .post(&format!("{}/rtpengine/answer", self.api.gateway()))
            .header("Content-Type", "application/sdp")
            .header("Authorization", format!("Bearer {}", token))
            .body(self.offer.clone())
            .send()
            .await?;

        let status = res.status().as_u16();

        if status == 201 {
            let endpoint = res.headers().get("Location").ok_or(MediaEngineError::MissingLocation)?;
            let location = endpoint.to_str().map_err(|_e| MediaEngineError::InvalidLocation)?.to_string();
            let sdp: Bytes = res.bytes().await?;
            log::info!("[MediaRtpEngineAnswer] created answer {location}");
            self.created = Some((location, sdp.clone()));
            Ok(sdp)
        } else {
            let response = res.text().await?;
            log::error!("[MediaRtpEngineAnswer] create answer error {status}, {response}");
            Err(MediaEngineError::InvalidStatus(status))
        }
    }
}

impl Drop for MediaRtpEngineAnswer {
    fn drop(&mut self) {
        if let Some((location, _)) = self.created.take() {
            let url = format!("{}{}", self.api.gateway(), location);
            tokio::spawn(async move {
                log::info!("[MediaRtpEngineAnswer] destroying {url}");
                let res = reqwest::ClientBuilder::new()
                    .timeout(Duration::from_secs(3))
                    .build()
                    .expect("Should create client")
                    .delete(&url)
                    .send()
                    .await?;

                let status = res.status().as_u16();
                if status == 200 {
                    log::info!("[MediaRtpEngineAnswer] destroyed {url}");
                    Ok(())
                } else {
                    log::error!("[MediaRtpEngineAnswer] destroy error {url} {status}");
                    Err(MediaEngineError::InvalidStatus(status))
                }
            });
        }
    }
}
