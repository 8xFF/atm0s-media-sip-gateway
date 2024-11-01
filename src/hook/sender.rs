use std::{collections::HashMap, marker::PhantomData, time::Duration};

use prost::Message;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::protocol::HookContentType;

use super::queue::HttpHookRequest;

pub struct HttpHookSender<Event> {
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub tx: UnboundedSender<HttpHookRequest<Event>>,
    pub _tmp: PhantomData<Event>,
}

impl<Event: Serialize + Message> HttpHookSender<Event> {
    pub fn send(&self, content_type: HookContentType, body: Event) {
        self.tx
            .send(HttpHookRequest {
                endpoint: self.endpoint.clone(),
                headers: self.headers.clone(),
                body,
                content_type,
                res_tx: None,
            })
            .expect("should send to queue worker");
    }

    pub async fn request<Res: DeserializeOwned + Message + Default>(&self, content_type: HookContentType, body: &Event) -> anyhow::Result<Res> {
        let client = reqwest::ClientBuilder::new().timeout(Duration::from_secs(5)).build().expect("should create client");
        let (content_type_str, body) = match content_type {
            HookContentType::Json => ("application/json", serde_json::to_vec(&body).expect("should convert to json")),
            HookContentType::Protobuf => ("application/protobuf", body.encode_to_vec()),
        };

        let mut builder = client.post(&self.endpoint).body(body).header("Content-Type", content_type_str);
        for (k, v) in &self.headers {
            builder = builder.header(k, v);
        }
        let res = builder.send().await?.error_for_status()?;
        match content_type {
            HookContentType::Json => Ok(res.json::<Res>().await?),
            HookContentType::Protobuf => {
                let binary = res.bytes().await?;
                Ok(Res::decode(binary)?)
            }
        }
    }
}
