use std::hash::{Hash, Hasher};

use atm0s_small_p2p::pubsub_service::PubsubChannelId;
use serde::{Deserialize, Serialize};

use super::AppId;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct NotifyIdentify {
    pub app: AppId,
    pub client: String,
}

impl NotifyIdentify {
    pub fn to_pubsub_channel(&self) -> PubsubChannelId {
        let mut hasher = std::hash::DefaultHasher::default();
        self.app.hash(&mut hasher);
        self.client.hash(&mut hasher);
        hasher.finish().into()
    }
}
