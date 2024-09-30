use serde::Serialize;

use super::{CallActionRequest, InternalCallId};

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum IncomingCallSipEvent {
    Cancelled,
    Bye,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "content")]
pub enum IncomingCallEvent {
    Accepted,
    Sip(IncomingCallSipEvent),
    Error { message: String },
    Destroyed,
}

#[derive(Debug, Serialize)]
pub struct HookIncomingCallRequest {
    pub gateway: String,
    pub call_id: InternalCallId,
    pub call_token: String,
    pub call_ws: String,
    pub from: String,
    pub to: String,
}

pub type HookIncomingCallResponse = CallActionRequest;
