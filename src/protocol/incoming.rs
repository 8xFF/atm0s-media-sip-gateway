use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

use super::{
    protobuf::sip_gateway::incoming_call_data::{incoming_call_event, incoming_call_request, incoming_call_response},
    InternalCallId, StreamingInfo,
};

#[derive(Debug, Serialize)]
pub struct HookIncomingCallRequest {
    pub call_id: InternalCallId,
    pub call_token: String,
    pub call_ws: String,
    pub from: String,
    pub to: String,
}

pub type HookIncomingCallResponse = IncomingCallActionRequest;

#[derive(Debug, Enum, Serialize, Deserialize)]
pub enum IncomingCallAction {
    Ring,
    Accept,
    End,
}

#[derive(Debug, Object, Serialize, Deserialize)]
pub struct IncomingCallActionRequest {
    pub action: IncomingCallAction,
    pub stream: Option<StreamingInfo>,
}

impl TryFrom<IncomingCallActionRequest> for incoming_call_request::Action {
    type Error = &'static str;

    fn try_from(mut value: IncomingCallActionRequest) -> Result<Self, Self::Error> {
        let req = match value.action {
            IncomingCallAction::Ring => incoming_call_request::Action::Ring(incoming_call_request::Ring {}),
            IncomingCallAction::Accept => {
                let stream = value.stream.take().ok_or("missing stream info")?;
                incoming_call_request::Action::Accept(incoming_call_request::Accept {
                    room: stream.room,
                    peer: stream.peer,
                    record: stream.record,
                })
            }
            IncomingCallAction::End => incoming_call_request::Action::End(incoming_call_request::End {}),
        };
        Ok(req)
    }
}

#[derive(Debug, Object, Serialize, Deserialize)]
pub struct IncomingCallActionResponse {}

impl TryFrom<incoming_call_response::Response> for IncomingCallActionResponse {
    type Error = String;

    fn try_from(value: incoming_call_response::Response) -> Result<Self, Self::Error> {
        match value {
            incoming_call_response::Response::Error(error) => Err(error.message),
            _ => Ok(IncomingCallActionResponse {}),
        }
    }
}

pub fn is_sip_incoming_cancelled(event: &Option<incoming_call_event::Event>) -> Option<()> {
    match event.as_ref()? {
        incoming_call_event::Event::Err(..) => None,
        incoming_call_event::Event::Sip(sip_event) => match &sip_event.event? {
            incoming_call_event::sip_event::Event::Cancelled(..) => Some(()),
            incoming_call_event::sip_event::Event::Bye(..) => None,
        },
        incoming_call_event::Event::Accepted(..) => None,
        incoming_call_event::Event::Ended(..) => None,
    }
}

pub fn is_sip_incoming_accepted(event: &Option<incoming_call_event::Event>) -> Option<()> {
    match event.as_ref()? {
        incoming_call_event::Event::Err(..) => None,
        incoming_call_event::Event::Sip(..) => None,
        incoming_call_event::Event::Accepted(..) => Some(()),
        incoming_call_event::Event::Ended(..) => None,
    }
}
