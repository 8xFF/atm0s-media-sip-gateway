use std::hash::{Hash, Hasher};

use atm0s_small_p2p::pubsub_service::PubsubChannelId;
use derive_more::derive::{Deref, Display, From, Into};
use ipnet::IpNet;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod incoming;
mod outgoing;
pub mod protobuf;
mod token;

pub use incoming::*;
pub use outgoing::*;
pub use token::*;

/// Note that his call_id is from internal state and not a SipCallID
#[derive(Debug, From, Into, Deref, Clone, Display, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalCallId(String);

impl InternalCallId {
    pub fn random() -> Self {
        Self(rand::random::<u64>().to_string())
    }

    pub fn to_pubsub_channel(&self) -> PubsubChannelId {
        let mut hasher = std::hash::DefaultHasher::default();
        self.hash(&mut hasher);
        hasher.finish().into()
    }
}

#[derive(Debug, Object)]
pub struct SipAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Object, Serialize, Deserialize)]
pub struct StreamingInfo {
    pub room: String,
    pub peer: String,
    pub record: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PhoneNumber {
    pub number: String,
    pub subnets: Vec<IpNet>,
    pub hook: String,
    pub app_secret: String,
}

#[derive(Error, Debug)]
pub enum CallApiError {
    #[error("BadRequest {0}")]
    BadRequest(&'static str),
    #[error("InternalChannel {0}")]
    InternalChannel(String),
    #[error("WrongSecret")]
    WrongSecret,
    #[error("WrongToken")]
    WrongToken,
    #[error("SipError {0}")]
    SipError(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum CallDirection {
    Outgoing,
    Incoming,
}
