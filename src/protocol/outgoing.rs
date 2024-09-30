use poem_openapi::{Enum, Object};
use serde::Serialize;

use super::{SipAuth, StreamingInfo};

#[derive(Debug, Enum)]
pub enum OutgoingCallAction {
    End,
}

#[derive(Debug, Object)]
pub struct CreateCallRequest {
    pub sip_server: String,
    pub sip_auth: Option<SipAuth>,
    pub from_number: String,
    pub to_number: String,
    pub hook: String,
    pub streaming: StreamingInfo,
}

#[derive(Debug, Object)]
pub struct CreateCallResponse {
    pub gateway: String,
    pub call_id: String,
    pub call_token: String,
    pub call_ws: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum OutgoingCallSipEvent {
    Provisional { code: u16 },
    Early { code: u16 },
    Accepted { code: u16 },
    Failure { code: u16 },
    Bye {},
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum OutgoingCallEvent {
    Sip(OutgoingCallSipEvent),
    Error { message: String },
    Destroyed,
}
