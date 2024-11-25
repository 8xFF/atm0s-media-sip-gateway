use std::time::Duration;

use bytes::Bytes;

use crate::protocol::StreamingInfo;

use super::{MediaApi, MediaEngineError};

pub struct MediaRtpEngineOffer {
    api: MediaApi,
    stream: StreamingInfo,
    offer: Option<(String, Bytes)>,
    answered: bool,
}

impl MediaRtpEngineOffer {
    pub fn new(api: MediaApi, stream: StreamingInfo) -> Self {
        Self {
            api,
            stream,
            offer: None,
            answered: false,
        }
    }

    pub fn sdp(&self) -> Option<Bytes> {
        self.offer.as_ref().map(|(_, sdp)| sdp.clone())
    }

    pub fn answered(&self) -> bool {
        self.answered
    }

    pub async fn create_offer(&mut self) -> Result<Bytes, MediaEngineError> {
        assert!(self.offer.is_none(), "should not call create_offer twice");
        log::info!("[RtpEngineOffer] creating token");
        let token = self.api.create_rtpengine_token(&self.stream.room, &self.stream.peer, self.stream.record).await?;
        log::info!("[RtpEngineOffer] created token");
        log::info!("[RtpEngineOffer] creating offer");
        let res = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Should create client")
            .post(format!("{}/rtpengine/offer", self.api.gateway()))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        let status = res.status().as_u16();

        if status == 201 {
            let endpoint = res.headers().get("Location").ok_or(MediaEngineError::MissingLocation)?;
            let location = endpoint.to_str().map_err(|_e| MediaEngineError::InvalidLocation)?.to_string();
            let sdp = res.bytes().await?;
            log::info!("[RtpEngineOffer] created offer {location}");
            self.offer = Some((location, sdp.clone()));
            Ok(sdp)
        } else {
            log::error!("[RtpEngineOffer] create offer error {status}");
            Err(MediaEngineError::InvalidStatus(status))
        }
    }

    pub async fn set_answer(&mut self, sdp: Bytes) -> Result<(), MediaEngineError> {
        let (location, _) = self.offer.as_ref().expect("should call after create_offer success");
        let url = format!("{}{}", self.api.gateway(), location);
        log::info!("[RtpEngineOffer] sending answer {url}");

        let res = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Should create client")
            .patch(&url)
            .header("Content-Type", "application/sdp")
            .body(sdp)
            .send()
            .await?;

        let status = res.status().as_u16();
        if status == 200 {
            log::info!("[RtpEngineOffer] sent answer {url}");
            self.answered = true;
            Ok(())
        } else {
            log::error!("[RtpEngineOffer] send answer error {url} {status}");
            Err(MediaEngineError::InvalidStatus(status))
        }
    }
}

impl Drop for MediaRtpEngineOffer {
    fn drop(&mut self) {
        if let Some((location, _)) = self.offer.take() {
            let url = format!("{}{}", self.api.gateway(), location);
            tokio::spawn(async move {
                log::info!("[RtpEngineOffer] destroying {url}");
                let res = reqwest::ClientBuilder::new()
                    .timeout(Duration::from_secs(3))
                    .build()
                    .expect("Should create client")
                    .delete(&url)
                    .send()
                    .await?;

                let status = res.status().as_u16();
                if status == 200 {
                    log::info!("[RtpEngineOffer] destroyed {url}");
                    Ok(())
                } else {
                    log::error!("[RtpEngineOffer] destroy error {url} {status}");
                    Err(MediaEngineError::InvalidStatus(status))
                }
            });
        }
    }
}
