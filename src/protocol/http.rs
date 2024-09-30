use derive_more::derive::From;
use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

use super::StreamingInfo;

#[derive(Debug, Enum, Serialize, Deserialize)]
pub enum CallAction {
    Trying,
    Ring,
    Accept,
    Reject,
}

#[derive(Debug, Object, Serialize, Deserialize)]
pub struct CallActionRequest {
    pub action: CallAction,
    pub stream: Option<StreamingInfo>,
}

#[derive(Debug, Object, Serialize, Deserialize)]
pub struct CallActionResponse {}

#[derive(Debug, Serialize, Deserialize)]
pub struct WsActionRequest {
    pub request_id: u32,
    pub request: CallActionRequest,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WsActionResponse {
    pub request_id: Option<u32>,
    pub success: bool,
    pub message: Option<String>,
    pub response: Option<CallActionResponse>,
}

#[derive(Debug, From, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum WsMessage {
    Event(serde_json::Value),
    Request(WsActionRequest),
    Response(WsActionResponse),
}
