use std::{collections::HashMap, time::Duration};

use bytes::Bytes;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct HttpHookRequest {
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

struct HttpHookQueue {
    rx: UnboundedReceiver<HttpHookRequest>,
}

impl HttpHookQueue {
    async fn send(&mut self, req: &HttpHookRequest) -> Result<(), reqwest::Error> {
        let client = reqwest::ClientBuilder::new().timeout(Duration::from_secs(10)).build().expect("should create client");

        let mut builder = client.post(&req.endpoint).body(req.body.clone()).header("Content-Type", "application/json");
        for (k, v) in &req.headers {
            builder = builder.header(k, v);
        }
        builder.send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn run(&mut self) {
        let req = self.rx.recv().await.expect("should receive");
        log::info!("[HttpHookQueue] sending hook to {}", req.endpoint);
        if let Err(e) = self.send(&req).await {
            log::error!("[HttpHookQueue] send hook to {} error {e:?}", req.endpoint);
        } else {
            log::info!("[HttpHookQueue] sent hook to {}", req.endpoint);
        }
    }
}

pub fn new_queue() -> UnboundedSender<HttpHookRequest> {
    let (tx, rx) = unbounded_channel();
    let mut queue = HttpHookQueue { rx };
    tokio::spawn(async move {
        loop {
            queue.run().await;
        }
    });

    tx
}
