use std::{collections::HashMap, marker::PhantomData, time::Duration};

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use super::queue::HttpHookRequest;

pub struct HttpHookSender<Event> {
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub tx: UnboundedSender<HttpHookRequest>,
    pub _tmp: PhantomData<Event>,
}

impl<Event: Serialize> HttpHookSender<Event> {
    pub fn send(&self, body: &Event) {
        self.tx
            .send(HttpHookRequest {
                endpoint: self.endpoint.clone(),
                headers: self.headers.clone(),
                body: serde_json::to_vec(body).expect("should convert to json").into(),
            })
            .expect("should send to queue worker");
    }

    pub async fn request<Res: DeserializeOwned>(&self, body: &Event) -> anyhow::Result<Res> {
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
