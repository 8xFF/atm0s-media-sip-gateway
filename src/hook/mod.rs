use std::{collections::HashMap, marker::PhantomData};

use prost::Message;
use queue::{new_queue, HttpHookRequest};
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;

mod queue;
mod sender;

pub use sender::HttpHookSender;

pub struct HttpHook<Event> {
    queues: Vec<UnboundedSender<HttpHookRequest<Event>>>,
}

impl<Event> HttpHook<Event>
where
    Event: Serialize + Message + Send + Sync + Default + 'static,
{
    pub fn new(size: usize) -> Self {
        let mut queues = vec![];
        for _ in 0..size {
            let queue_tx = new_queue();
            queues.push(queue_tx);
        }
        Self { queues }
    }

    pub fn new_sender(&self, endpoint: &str, headers: HashMap<String, String>) -> HttpHookSender<Event> {
        let index = rand::random::<usize>() % self.queues.len();
        HttpHookSender {
            endpoint: endpoint.to_owned(),
            headers,
            tx: self.queues[index].clone(),
            _tmp: PhantomData,
        }
    }
}
