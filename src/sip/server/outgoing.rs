use std::io;

use calling_state::CallingState;
use early_state::EarlyState;
use ezk_sip_auth::{
    digest::{DigestAuthenticator, DigestCredentials},
    CredentialStore, UacAuthSession,
};
use ezk_sip_core::{Endpoint, LayerKey};
use ezk_sip_types::{header::typed::Contact, uri::NameAddr};
use ezk_sip_ua::{
    dialog::DialogLayer,
    invite::{initiator::Initiator, InviteLayer},
};
use talking_state::TalkingState;
use thiserror::Error;

use crate::{
    protocol::{
        protobuf::sip_gateway::outgoing_call_data::{
            outgoing_call_event::{self, sip_event},
            OutgoingCallEvent,
        },
        InternalCallId, SipAuth, StreamingInfo,
    },
    sip::{MediaApi, MediaEngineError, MediaRtpEngineOffer},
};

mod calling_state;
mod early_state;
mod talking_state;

enum StateOut {
    Event(OutgoingCallEvent),
    Switch(State, OutgoingCallEvent),
    Continue,
}

trait StateLogic {
    fn start(&mut self, ctx: &mut Ctx) -> impl std::future::Future<Output = Result<(), SipOutgoingCallError>>;
    fn end(&mut self, ctx: &mut Ctx) -> impl std::future::Future<Output = Result<(), SipOutgoingCallError>>;
    fn recv(&mut self, ctx: &mut Ctx) -> impl std::future::Future<Output = Result<Option<StateOut>, SipOutgoingCallError>>;
}

struct OutgoingAuth {
    session: UacAuthSession,
    credentials: CredentialStore,
}

enum State {
    Calling(CallingState),
    Early(EarlyState),
    Talking(TalkingState),
}

impl StateLogic for State {
    async fn start(&mut self, ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        match self {
            State::Calling(state) => state.start(ctx).await,
            State::Early(state) => state.start(ctx).await,
            State::Talking(state) => state.start(ctx).await,
        }
    }
    async fn end(&mut self, ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        match self {
            State::Calling(state) => state.end(ctx).await,
            State::Early(state) => state.end(ctx).await,
            State::Talking(state) => state.end(ctx).await,
        }
    }
    async fn recv(&mut self, ctx: &mut Ctx) -> Result<Option<StateOut>, SipOutgoingCallError> {
        match self {
            State::Calling(state) => state.recv(ctx).await,
            State::Early(state) => state.recv(ctx).await,
            State::Talking(state) => state.recv(ctx).await,
        }
    }
}

#[derive(Error, Debug)]
pub enum SipOutgoingCallError {
    #[error("IoError({0})")]
    Io(#[from] io::Error),
    #[error("EzkCoreError({0})")]
    EzkCore(#[from] ezk_sip_core::Error),
    #[error("EzkAuthError({0})")]
    EzkAuth(#[from] ezk_sip_auth::Error),
    #[error("RtpEngine{0}")]
    RtpEngine(#[from] MediaEngineError),
    #[error("ParseError{0}")]
    Parse(String),
}

pub enum SipOutgoingCallOut {
    Event(OutgoingCallEvent),
    Continue,
}

struct Ctx {
    call_id: InternalCallId,
    initiator: Initiator,
    auth: Option<OutgoingAuth>,
    rtp: MediaRtpEngineOffer,
}

pub struct SipOutgoingCall {
    ctx: Ctx,
    state: State,
}

impl SipOutgoingCall {
    pub fn new(
        media_api: MediaApi,
        endpoint: Endpoint,
        dialog_layer: LayerKey<DialogLayer>,
        invite_layer: LayerKey<InviteLayer>,
        from: &str,
        to: &str,
        contact: Contact,
        auth: Option<SipAuth>,
        stream: StreamingInfo,
    ) -> Result<Self, SipOutgoingCallError> {
        let call_id: InternalCallId = InternalCallId::random();
        log::info!("[SipOutgoingCall {call_id}] create with {from} => {to}");
        let local_uri = endpoint.parse_uri(from).map_err(|e| SipOutgoingCallError::Parse(e.to_string()))?;
        let target = endpoint.parse_uri(to).map_err(|e| SipOutgoingCallError::Parse(e.to_string()))?;

        let initiator = Initiator::new(endpoint, dialog_layer, invite_layer, NameAddr::uri(local_uri.clone()), contact, target);

        let auth = auth.map(|auth| {
            let mut credentials = CredentialStore::new();
            credentials.set_default(DigestCredentials::new(auth.username, auth.password));
            OutgoingAuth {
                session: UacAuthSession::new(DigestAuthenticator::default()),
                credentials,
            }
        });

        Ok(Self {
            ctx: Ctx {
                initiator,
                auth,
                call_id,
                rtp: MediaRtpEngineOffer::new(media_api, stream),
            },
            state: State::Calling(CallingState::default()),
        })
    }

    pub fn call_id(&self) -> InternalCallId {
        self.ctx.call_id.clone()
    }

    pub async fn start(&mut self) -> Result<(), SipOutgoingCallError> {
        self.state.start(&mut self.ctx).await
    }

    pub async fn end(&mut self) -> Result<(), SipOutgoingCallError> {
        self.state.end(&mut self.ctx).await
    }

    pub async fn recv(&mut self) -> Result<Option<SipOutgoingCallOut>, SipOutgoingCallError> {
        match self.state.recv(&mut self.ctx).await? {
            Some(out) => match out {
                StateOut::Event(event) => Ok(Some(SipOutgoingCallOut::Event(event))),
                StateOut::Switch(state, event) => {
                    self.state = state;
                    Ok(Some(SipOutgoingCallOut::Event(event)))
                }
                StateOut::Continue => Ok(Some(SipOutgoingCallOut::Continue)),
            },
            None => Ok(None),
        }
    }
}

fn build_sip_event(event: sip_event::Event) -> OutgoingCallEvent {
    OutgoingCallEvent {
        event: Some(outgoing_call_event::Event::Sip(outgoing_call_event::SipEvent { event: Some(event) })),
    }
}
