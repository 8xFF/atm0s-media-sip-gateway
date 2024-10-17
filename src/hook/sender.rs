use std::{collections::HashMap, time::Duration};

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use super::queue::HttpHookRequest;

pub struct HttpHookSender {
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub tx: UnboundedSender<HttpHookRequest>,
}

impl HttpHookSender {
    pub fn send<E: Serialize>(&self, body: &E) {
        self.tx
            .send(HttpHookRequest {
                endpoint: self.endpoint.clone(),
                headers: self.headers.clone(),
                body: serde_json::to_vec(body).expect("should convert to json").into(),
            })
            .expect("should send to queue worker");
    }

    pub async fn request<Req: Serialize, Res: DeserializeOwned>(&self, body: &Req) -> anyhow::Result<Res> {
        let client = reqwest::ClientBuilder::new().timeout(Duration::from_secs(5)).build().expect("should create client");
        let body_str = serde_json::to_vec(body).expect("should convert to json");

        let mut builder = client.post(&self.endpoint).body(body_str).header("Content-Type", "application/json");
        for (k, v) in &self.headers {
            builder = builder.header(k, v);
        }
        let res = builder.send().await?.error_for_status()?.json::<Res>().await?;
        Ok(res)
    }
}

pub struct HttpHookSenderNoContext {
    pub headers: HashMap<String, String>,
    pub tx: UnboundedSender<HttpHookRequest>,
}

impl HttpHookSenderNoContext {
    pub fn send<E: Serialize>(&self, endpoint: &str, body: &E) {
        self.tx
            .send(HttpHookRequest {
                endpoint: endpoint.to_owned(),
                headers: self.headers.clone(),
                body: serde_json::to_vec(body).expect("should convert to json").into(),
            })
            .expect("should send to queue worker");
    }

    pub async fn request<Req: Serialize, Res: DeserializeOwned>(&self, endpoint: &str, body: &Req) -> anyhow::Result<Res> {
        let client = reqwest::ClientBuilder::new().timeout(Duration::from_secs(5)).build().expect("should create client");
        let body_str = serde_json::to_vec(body).expect("should convert to json");

        let mut builder = client.post(endpoint).body(body_str).header("Content-Type", "application/json");
        for (k, v) in &self.headers {
            builder = builder.header(k, v);
        }
        let res = builder.send().await?.error_for_status()?.json::<Res>().await?;
        Ok(res)
    }
}
