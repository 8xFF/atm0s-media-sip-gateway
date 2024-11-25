use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use atm0s_small_p2p::pubsub_service::PubsubServiceRequester;
use incoming_call::IncomingCall;
use outgoing_call::OutgoingCall;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::{
    address_book::AddressBookStorage,
    hook::HttpHook,
    protocol::{protobuf::sip_gateway::CallEvent, CallApiError, CallDirection, CreateCallRequest, CreateCallResponse, InternalCallId},
    secure::{CallToken, SecureContext},
    sip::{MediaApi, SipServer},
    utils::select2,
};

pub mod incoming_call;
pub mod outgoing_call;

pub enum CallManagerOut {
    Continue,
    IncomingCall(),
}

pub struct CallManager {
    call_pubsub: PubsubServiceRequester,
    sip: SipServer,
    http_hook: HttpHook<CallEvent>,
    out_calls: HashMap<InternalCallId, OutgoingCall>,
    in_calls: HashMap<InternalCallId, IncomingCall>,
    destroy_tx: UnboundedSender<InternalCallId>,
    destroy_rx: UnboundedReceiver<InternalCallId>,
    secure_ctx: Arc<SecureContext>,
    address_book: AddressBookStorage,
    media_gateway: String,
}

impl CallManager {
    pub async fn new(
        call_pubsub: PubsubServiceRequester,
        sip_listen: SocketAddr,
        public_ip: IpAddr,
        address_book: AddressBookStorage,
        secure_ctx: Arc<SecureContext>,
        http_hook: HttpHook<CallEvent>,
        media_gateway: &str,
    ) -> Self {
        let sip = SipServer::new(sip_listen, public_ip).await.expect("should create sip-server");
        let (destroy_tx, destroy_rx) = unbounded_channel();
        Self {
            call_pubsub,
            sip,
            http_hook,
            out_calls: HashMap::new(),
            in_calls: HashMap::new(),
            destroy_tx,
            destroy_rx,
            secure_ctx,
            address_book,
            media_gateway: media_gateway.to_owned(),
        }
    }

    pub fn create_call(&mut self, req: CreateCallRequest, media_api: MediaApi) -> Result<CreateCallResponse, CallApiError> {
        let hook_sender = self.http_hook.new_sender(&req.hook, HashMap::new());
        let from = format!("sip:{}@{}", req.from_number, req.sip_server);
        let to = format!("sip:{}@{}", req.to_number, req.sip_server);
        match self.sip.make_call(media_api, &from, &to, req.sip_auth, req.streaming) {
            Ok(call) => {
                let call_id = call.call_id();
                let call_token = self.secure_ctx.encode_call_token(
                    CallToken {
                        direction: CallDirection::Outgoing,
                        call_id: call_id.clone(),
                    },
                    3600,
                );
                self.out_calls.insert(
                    call_id.clone(),
                    OutgoingCall::new(call, self.destroy_tx.clone(), req.hook_content_type, hook_sender, self.call_pubsub.clone()),
                );
                Ok(CreateCallResponse {
                    call_ws: format!("/call/outgoing/{call_id}?token={call_token}"),
                    call_id: call_id.clone().into(),
                    call_token,
                })
            }
            Err(err) => Err(CallApiError::SipError(err.to_string())),
        }
    }

    pub async fn recv(&mut self) -> Option<CallManagerOut> {
        let out = select2::or(self.destroy_rx.recv(), self.sip.recv()).await;
        match out {
            select2::OrOutput::Left(call_id) => {
                let call_id = call_id?;
                if self.out_calls.remove(&call_id).is_none() && self.in_calls.remove(&call_id).is_none() {
                    log::warn!("[CallManager] got Destroyed event for {call_id} but not found");
                }
                Some(CallManagerOut::Continue)
            }
            select2::OrOutput::Right(event) => match event? {
                crate::sip::SipServerOut::Incoming(call) => {
                    if let Some((app, number)) = self.address_book.validate_phone(call.remote(), call.from(), call.to()) {
                        let hook_sender = self.http_hook.new_sender(&number.hook, HashMap::new());
                        let call_id = call.call_id();
                        let call_token = self.secure_ctx.encode_call_token(
                            CallToken {
                                direction: CallDirection::Outgoing,
                                call_id: call_id.clone(),
                            },
                            3600,
                        );
                        let api: MediaApi = MediaApi::new(&self.media_gateway, &app.app_secret);
                        let call = IncomingCall::new(api, call, call_token, self.destroy_tx.clone(), number.hook_content_type, hook_sender, self.call_pubsub.clone());
                        self.in_calls.insert(call_id, call);
                        Some(CallManagerOut::IncomingCall())
                    } else {
                        log::warn!("[CallManager] rejected call from server {} with number {} => {}", call.remote(), call.from(), call.to());
                        call.kill_because_validate_failed();
                        Some(CallManagerOut::Continue)
                    }
                }
            },
        }
    }
}
