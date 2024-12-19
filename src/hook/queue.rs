use std::{collections::HashMap, time::Duration};

use prost::Message;
use serde::Serialize;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::protocol::HookContentType;

const MAX_RETRY: usize = 5;

pub struct HttpHookRequest<Event> {
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub body: Event,
    pub content_type: HookContentType,
    pub res_tx: Option<oneshot::Sender<reqwest::Result<reqwest::Response>>>,
}

struct HttpHookQueue<Event> {
    rx: UnboundedReceiver<HttpHookRequest<Event>>,
}

impl<Event: Serialize + Message> HttpHookQueue<Event> {
    async fn send(&mut self, req: &HttpHookRequest<Event>) -> reqwest::Result<reqwest::Response> {
        let client = reqwest::ClientBuilder::new().timeout(Duration::from_secs(10)).build().expect("should create client");

        let (content_type, body) = match req.content_type {
            HookContentType::Json => ("application/json", serde_json::to_vec(&req.body).expect("should convert to json")),
            HookContentType::Protobuf => ("application/protobuf", req.body.encode_to_vec()),
        };

        let mut builder = client.post(&req.endpoint).body(body).header("Content-Type", content_type);
        for (k, v) in &req.headers {
            builder = builder.header(k, v);
        }
        builder.send().await?.error_for_status()
    }

    pub async fn run(&mut self) {
        let mut req = self.rx.recv().await.expect("should receive");
        let mut count = 0;
        while count < MAX_RETRY {
            count += 1;
            log::info!("[HttpHookQueue] sending hook to {}, retry {count}/{MAX_RETRY}", req.endpoint);
            let res = self.send(&req).await;
            match &res {
                Ok(_res) => {
                    log::info!("[HttpHookQueue] sent hook to {} in try {count}/{MAX_RETRY}", req.endpoint);
                }
                Err(e) => {
                    log::error!("[HttpHookQueue] send hook to {} error {e:?} in try {count}/{MAX_RETRY}", req.endpoint);
                    if e.is_connect() && count < MAX_RETRY {
                        log::error!("[HttpHookQueue] connect error => retry in try {count}/{MAX_RETRY}");
                        continue;
                    }
                }
            }
            if let Some(tx) = req.res_tx.take() {
                let _ = tx.send(res);
            }
            break;
        }
    }
}

pub fn new_queue<Event: Serialize + Message + Send + Sync + 'static>() -> UnboundedSender<HttpHookRequest<Event>> {
    let (tx, rx) = unbounded_channel();
    let mut queue = HttpHookQueue { rx };
    tokio::spawn(async move {
        loop {
            queue.run().await;
        }
    });

    tx
}
