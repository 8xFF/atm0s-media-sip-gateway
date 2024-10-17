use ezk_sip_ua::invite::session::Session;

use crate::{
    protocol::{
        protobuf::sip_gateway::incoming_call_data::{
            incoming_call_event::{self, sip_event},
            IncomingCallEvent,
        },
        StreamingInfo,
    },
    sip::{media::MediaRtpEngineAnswer, MediaApi},
};

use super::{Ctx, SipIncomingCallError, StateLogic, StateOut};

pub struct TalkingState {
    session: Session,
    _rtp: MediaRtpEngineAnswer,
}

impl TalkingState {
    pub fn new(session: Session, rtp: MediaRtpEngineAnswer) -> Self {
        Self { session, _rtp: rtp }
    }
}

impl StateLogic for TalkingState {
    async fn send_trying(&mut self, _ctx: &mut Ctx) -> Result<(), SipIncomingCallError> {
        Err(SipIncomingCallError::WrongState("Talking state cannot send trying"))
    }

    async fn send_ringing(&mut self, _ctx: &mut Ctx) -> Result<(), SipIncomingCallError> {
        Err(SipIncomingCallError::WrongState("Talking state cannot send ringing"))
    }

    async fn accept(&mut self, _ctx: &mut Ctx, _api: MediaApi, _stream: StreamingInfo) -> Result<(), SipIncomingCallError> {
        Err(SipIncomingCallError::WrongState("Talking state cannot send accept"))
    }

    async fn end(&mut self, _ctx: &mut Ctx) -> Result<(), SipIncomingCallError> {
        log::info!("[TalkingState] terminate session");
        self.session.terminate().await?;
        Ok(())
    }

    fn kill_because_validate_failed(self, _ctx: &mut Ctx) {
        panic!("should not call on talking state")
    }

    async fn recv(&mut self, _ctx: &mut Ctx) -> Result<Option<StateOut>, SipIncomingCallError> {
        match self.session.drive().await? {
            ezk_sip_ua::invite::session::Event::RefreshNeeded(_refresh_needed) => Ok(Some(StateOut::Continue)),
            ezk_sip_ua::invite::session::Event::ReInviteReceived(_re_invite_received) => Ok(Some(StateOut::Continue)),
            ezk_sip_ua::invite::session::Event::Bye(_) => {
                log::info!("[TalkingState] on Bye");
                Ok(Some(StateOut::Event(IncomingCallEvent {
                    event: Some(incoming_call_event::Event::Sip(incoming_call_event::SipEvent {
                        event: Some(sip_event::Event::Bye(sip_event::Bye {})),
                    })),
                })))
            }
            ezk_sip_ua::invite::session::Event::Terminated => {
                log::info!("[TalkingState] on Terminated");
                Ok(None)
            }
        }
    }
}
