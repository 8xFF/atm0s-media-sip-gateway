use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

use super::{
    protobuf::sip_gateway::outgoing_call_data::{outgoing_call_request, outgoing_call_response},
    HookContentType, SipAuth, StreamingInfo,
};

#[derive(Debug, Object)]
pub struct CreateCallRequest {
    pub sip_server: String,
    pub sip_proxy: Option<String>,
    pub sip_auth: Option<SipAuth>,
    pub from_number: String,
    pub to_number: String,
    pub hook: String,
    pub hook_content_type: HookContentType,
    pub streaming: StreamingInfo,
}

#[derive(Debug, Object)]
pub struct CreateCallResponse {
    pub call_id: String,
    pub call_token: String,
    pub call_ws: String,
}

#[derive(Debug, Enum, Serialize, Deserialize)]
pub enum OutgoingCallAction {
    End,
}

#[derive(Debug, Object, Serialize, Deserialize)]
pub struct OutgoingCallActionRequest {
    pub action: OutgoingCallAction,
    pub stream: Option<StreamingInfo>,
}

impl TryFrom<OutgoingCallActionRequest> for outgoing_call_request::Action {
    type Error = &'static str;

    fn try_from(value: OutgoingCallActionRequest) -> Result<Self, Self::Error> {
        let req = match value.action {
            OutgoingCallAction::End => outgoing_call_request::Action::End(outgoing_call_request::End {}),
        };
        Ok(req)
    }
}

#[derive(Debug, Object, Serialize, Deserialize)]
pub struct OutgoingCallActionResponse {}

impl TryFrom<outgoing_call_response::Response> for OutgoingCallActionResponse {
    type Error = String;

    fn try_from(value: outgoing_call_response::Response) -> Result<Self, Self::Error> {
        match value {
            outgoing_call_response::Response::Error(error) => Err(error.message),
            outgoing_call_response::Response::End(_end) => Ok(OutgoingCallActionResponse {}),
        }
    }
}
