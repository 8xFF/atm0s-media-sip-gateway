use std::{net::SocketAddr, sync::Arc};

use anyhow::anyhow;
use ezk_sip_core::{Endpoint, IncomingRequest, Layer, LayerKey, MayTake};
use ezk_sip_types::{
    header::typed::Contact,
    uri::sip::{SipUri, UserPart},
    Method,
};
use ezk_sip_ua::{
    dialog::{Dialog, DialogLayer},
    invite::{acceptor::Acceptor, InviteLayer},
};
use talking_state::TalkingState;
use thiserror::Error;
use tokio::sync::{mpsc::Sender, Notify};
use wait_state::WaitState;

use crate::{
    protocol::{protobuf::sip_gateway::incoming_call_data::IncomingCallEvent, InternalCallId, StreamingInfo},
    sip::{MediaApi, MediaEngineError},
};

mod talking_state;
mod wait_state;

/// Custom layer which we use to accept incoming invites
pub struct InviteAcceptLayer {
    contact: Contact,
    dialog_layer: LayerKey<DialogLayer>,
    invite_layer: LayerKey<InviteLayer>,
    incoming_tx: Sender<SipIncomingCall>,
}

impl InviteAcceptLayer {
    pub fn new(incoming_tx: Sender<SipIncomingCall>, contact: Contact, dialog_layer: LayerKey<DialogLayer>, invite_layer: LayerKey<InviteLayer>) -> Self {
        Self {
            contact,
            dialog_layer,
            invite_layer,
            incoming_tx,
        }
    }

    async fn process(&self, endpoint: &Endpoint, request: MayTake<'_, IncomingRequest>) -> anyhow::Result<()> {
        let invite = if request.line.method == Method::INVITE {
            &request
        } else {
            return Ok(());
        };

        log::info!("[Incoming] {:?}", invite.base_headers.from.uri);
        let from: &SipUri = invite.base_headers.from.uri.uri.downcast_ref().ok_or(anyhow!("parse from_uri error"))?;
        let to: &SipUri = invite.base_headers.to.uri.uri.downcast_ref().ok_or(anyhow!("parse to_uri error"))?;
        let from = get_user(&from.user_part).ok_or(anyhow!("missing from user"))?;
        let to = get_user(&to.user_part).ok_or(anyhow!("missing to user"))?;
        let remote = invite.tp_info.source;
        let offer_sdp = invite.body.clone();

        let invite = request.take();
        let dialog = Dialog::new_server(endpoint.clone(), self.dialog_layer, &invite, self.contact.clone()).unwrap();

        let cancelled = Arc::new(Notify::new());
        let cancelled_c = cancelled.clone();
        let acceptor = Acceptor::new(
            dialog,
            self.invite_layer,
            invite,
            Some(Box::new(move || {
                cancelled_c.notify_one();
            })),
        )?;

        let call = SipIncomingCall {
            call_id: InternalCallId::random(),
            state: State::Wait(WaitState::new(acceptor, offer_sdp, cancelled)),
            remote,
            from,
            to,
            ctx: Ctx {},
        };
        self.incoming_tx.send(call).await.expect("should send call to main loop");
        Ok(())
    }
}

#[async_trait::async_trait]
impl Layer for InviteAcceptLayer {
    fn name(&self) -> &'static str {
        "invite-accept-layer"
    }

    async fn receive(&self, endpoint: &Endpoint, request: MayTake<'_, IncomingRequest>) {
        if let Err(e) = self.process(endpoint, request).await {
            log::error!("[InviteAcceptLayer] process incoming request error {e}");
        }
    }
}

#[derive(Error, Debug)]
pub enum SipIncomingCallError {
    #[error("EzkCoreError({0})")]
    EzkCore(#[from] ezk_sip_core::Error),
    #[error("EzkAcceptorError({0})")]
    EzkAcceptor(#[from] ezk_sip_ua::invite::acceptor::Error),
    #[error("RtpEngine({0})")]
    RtpEngine(#[from] MediaEngineError),
    #[error("WrongState({0})")]
    WrongState(&'static str),
}

pub enum SipIncomingCallOut {
    Event(IncomingCallEvent),
    Continue,
}

struct Ctx {}

enum StateOut {
    Event(IncomingCallEvent),
    Switch(State, IncomingCallEvent),
    Continue,
}

trait StateLogic {
    fn send_trying(&mut self, ctx: &mut Ctx) -> impl std::future::Future<Output = Result<(), SipIncomingCallError>>;
    fn send_ringing(&mut self, ctx: &mut Ctx) -> impl std::future::Future<Output = Result<(), SipIncomingCallError>>;
    fn accept(&mut self, ctx: &mut Ctx, api: MediaApi, stream: StreamingInfo) -> impl std::future::Future<Output = Result<(), SipIncomingCallError>>;
    fn end(&mut self, ctx: &mut Ctx) -> impl std::future::Future<Output = Result<(), SipIncomingCallError>>;
    fn kill_because_validate_failed(self, ctx: &mut Ctx);
    fn recv(&mut self, ctx: &mut Ctx) -> impl std::future::Future<Output = Result<Option<StateOut>, SipIncomingCallError>>;
}

enum State {
    Wait(WaitState),
    Talking(TalkingState),
}

impl StateLogic for State {
    async fn send_trying(&mut self, ctx: &mut Ctx) -> Result<(), SipIncomingCallError> {
        match self {
            State::Wait(state) => state.send_trying(ctx).await,
            State::Talking(state) => state.send_trying(ctx).await,
        }
    }

    async fn send_ringing(&mut self, ctx: &mut Ctx) -> Result<(), SipIncomingCallError> {
        match self {
            State::Wait(state) => state.send_ringing(ctx).await,
            State::Talking(state) => state.send_ringing(ctx).await,
        }
    }

    async fn accept(&mut self, ctx: &mut Ctx, api: MediaApi, stream: StreamingInfo) -> Result<(), SipIncomingCallError> {
        match self {
            State::Wait(state) => state.accept(ctx, api, stream).await,
            State::Talking(state) => state.accept(ctx, api, stream).await,
        }
    }

    async fn end(&mut self, ctx: &mut Ctx) -> Result<(), SipIncomingCallError> {
        match self {
            State::Wait(state) => state.end(ctx).await,
            State::Talking(state) => state.end(ctx).await,
        }
    }

    fn kill_because_validate_failed(self, ctx: &mut Ctx) {
        match self {
            State::Wait(state) => state.kill_because_validate_failed(ctx),
            State::Talking(state) => state.kill_because_validate_failed(ctx),
        }
    }

    async fn recv(&mut self, ctx: &mut Ctx) -> Result<Option<StateOut>, SipIncomingCallError> {
        match self {
            State::Wait(state) => state.recv(ctx).await,
            State::Talking(state) => state.recv(ctx).await,
        }
    }
}

pub struct SipIncomingCall {
    call_id: InternalCallId,
    remote: SocketAddr,
    from: String,
    to: String,
    state: State,
    ctx: Ctx,
}

impl SipIncomingCall {
    pub fn call_id(&self) -> InternalCallId {
        self.call_id.clone()
    }

    pub fn remote(&self) -> SocketAddr {
        self.remote
    }

    pub fn from(&self) -> &str {
        &self.from
    }

    pub fn to(&self) -> &str {
        &self.to
    }

    pub async fn send_trying(&mut self) -> Result<(), SipIncomingCallError> {
        self.state.send_trying(&mut self.ctx).await
    }

    pub async fn send_ringing(&mut self) -> Result<(), SipIncomingCallError> {
        self.state.send_ringing(&mut self.ctx).await
    }

    pub async fn accept(&mut self, api: MediaApi, stream: StreamingInfo) -> Result<(), SipIncomingCallError> {
        self.state.accept(&mut self.ctx, api, stream).await
    }

    pub async fn end(&mut self) -> Result<(), SipIncomingCallError> {
        self.state.end(&mut self.ctx).await
    }

    pub fn kill_because_validate_failed(mut self) {
        self.state.kill_because_validate_failed(&mut self.ctx);
    }

    pub async fn recv(&mut self) -> Result<Option<SipIncomingCallOut>, SipIncomingCallError> {
        match self.state.recv(&mut self.ctx).await? {
            Some(out) => match out {
                StateOut::Event(event) => Ok(Some(SipIncomingCallOut::Event(event))),
                StateOut::Switch(state, event) => {
                    self.state = state;
                    Ok(Some(SipIncomingCallOut::Event(event)))
                }
                StateOut::Continue => Ok(Some(SipIncomingCallOut::Continue)),
            },
            None => Ok(None),
        }
    }
}

fn get_user(user_part: &UserPart) -> Option<String> {
    match user_part {
        UserPart::Empty => None,
        UserPart::User(user) => Some(user.to_string()),
        UserPart::UserPw(user_pw) => Some(user_pw.user.to_string()),
    }
}
