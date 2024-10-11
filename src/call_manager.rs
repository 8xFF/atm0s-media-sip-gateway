use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use anyhow::anyhow;
use derive_more::derive::{Display, From};
use incoming_call::IncomingCall;
use outgoing_call::OutgoingCall;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{
    address_book::AddressBookStorage,
    error::PrintErrorDetails,
    hook::HttpHook,
    protocol::{CallActionRequest, CallActionResponse, CallApiError, CallDirection, CreateCallRequest, CreateCallResponse, InternalCallId, WsMessage},
    secure::{CallToken, SecureContext},
    sip::{MediaApi, SipServer},
    utils::http_to_ws,
    utils::select2,
};

pub mod incoming_call;
pub mod outgoing_call;

#[derive(From, PartialEq, Eq, Hash, Clone, Copy, Display)]
pub struct EmitterId(u64);

impl EmitterId {
    pub fn rand() -> Self {
        Self(rand::random())
    }
}

pub trait EventEmitter: Send + Sync + 'static {
    fn emitter_id(&self) -> EmitterId;
    fn fire(&mut self, event: WsMessage);
}

pub enum CallManagerOut {
    Continue,
    IncomingCall(),
}

pub struct CallManager<EM> {
    http_public: String,
    sip: SipServer,
    http_hook: HttpHook,
    out_calls: HashMap<InternalCallId, OutgoingCall<EM>>,
    in_calls: HashMap<InternalCallId, IncomingCall<EM>>,
    destroy_tx: UnboundedSender<InternalCallId>,
    destroy_rx: UnboundedReceiver<InternalCallId>,
    secure_ctx: Arc<SecureContext>,
    address_book: AddressBookStorage,
    media_gateway: String,
}

impl<EM: EventEmitter> CallManager<EM> {
    pub async fn new(sip_addr: SocketAddr, http_public: &str, address_book: AddressBookStorage, secure_ctx: Arc<SecureContext>, http_hook: HttpHook, media_gateway: &str) -> Self {
        let sip = SipServer::new(sip_addr).await.expect("should create sip-server");
        let (destroy_tx, destroy_rx) = unbounded_channel();
        Self {
            http_public: http_public.to_owned(),
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
                self.out_calls.insert(call_id.clone(), OutgoingCall::new(call, self.destroy_tx.clone(), hook_sender));
                Ok(CreateCallResponse {
                    gateway: self.http_public.clone(),
                    call_ws: format!("{}/ws/call/{call_id}?token={call_token}", http_to_ws(&self.http_public)),
                    call_id: call_id.clone().into(),
                    call_token,
                })
            }
            Err(err) => Err(CallApiError::SipError(err.to_string())),
        }
    }

    pub fn subscribe_call(&mut self, call: InternalCallId, emitter: EM) -> Result<(), CallApiError> {
        if let Some(call) = self.out_calls.get_mut(&call) {
            call.add_emitter(emitter);
            Ok(())
        } else if let Some(call) = self.in_calls.get_mut(&call) {
            call.add_emitter(emitter);
            Ok(())
        } else {
            Err(CallApiError::CallNotFound)
        }
    }

    pub fn unsubscribe_call(&mut self, call: InternalCallId, emitter: EmitterId) -> Result<(), CallApiError> {
        if let Some(call) = self.out_calls.get_mut(&call) {
            call.del_emitter(emitter);
            Ok(())
        } else if let Some(call) = self.in_calls.get_mut(&call) {
            call.del_emitter(emitter);
            Ok(())
        } else {
            Err(CallApiError::CallNotFound)
        }
    }

    pub fn action_call(&mut self, call: InternalCallId, req: CallActionRequest, tx: oneshot::Sender<anyhow::Result<CallActionResponse>>) {
        if let Some(_call) = self.out_calls.get_mut(&call) {
            tx.send(Err(anyhow!("action_call not working with outgoing call")))
                .print_error_detail("[CallManager] feedback action_call not working with outgoing call");
        } else if let Some(call) = self.in_calls.get_mut(&call) {
            call.do_action(req, tx);
        } else {
            tx.send(Err(anyhow!("call not found"))).print_error_detail("[CallManager] feedback action_call not found");
        };
    }

    pub fn end_call(&mut self, call: InternalCallId) -> Result<(), CallApiError> {
        if let Some(call) = self.out_calls.get_mut(&call) {
            call.end();
            Ok(())
        } else if let Some(call) = self.in_calls.get_mut(&call) {
            call.end();
            Ok(())
        } else {
            Err(CallApiError::CallNotFound)
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
                    if let Some(number) = self.address_book.allow(call.remote(), call.from(), call.to()) {
                        let hook_sender = self.http_hook.new_sender(&number.hook, HashMap::new());
                        let call_id = call.call_id();
                        let call_token = self.secure_ctx.encode_call_token(
                            CallToken {
                                direction: CallDirection::Outgoing,
                                call_id: call_id.clone(),
                            },
                            3600,
                        );
                        let api: MediaApi = MediaApi::new(&self.media_gateway, &number.app_secret);
                        let call = IncomingCall::new(&self.http_public, api, call, call_token, self.destroy_tx.clone(), hook_sender);
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
