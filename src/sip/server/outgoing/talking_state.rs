use std::collections::VecDeque;

use ezk_sip_ua::invite::session::Session;

use crate::{
    protocol::protobuf::sip_gateway::outgoing_call_data::{
        outgoing_call_event::{self, sip_event},
        OutgoingCallEvent,
    },
    sip::server::outgoing::build_sip_event,
};

use super::{Ctx, SipOutgoingCallError, StateLogic, StateOut};

#[derive(Debug)]
pub struct TalkingState {
    session: Session,
    outs: VecDeque<StateOut>,
}

impl TalkingState {
    pub fn new(session: Session) -> Self {
        Self { session, outs: VecDeque::new() }
    }
}

impl StateLogic for TalkingState {
    async fn start(&mut self, _ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        Ok(())
    }
    async fn end(&mut self, _ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        self.session.terminate().await?;
        self.outs.push_back(StateOut::Event(OutgoingCallEvent {
            event: Some(outgoing_call_event::Event::Terminated(Default::default())),
        }));
        Ok(())
    }
    async fn recv(&mut self, _ctx: &mut Ctx) -> Result<Option<StateOut>, SipOutgoingCallError> {
        if let Some(out) = self.outs.pop_front() {
            return Ok(Some(out));
        }
        match self.session.drive().await? {
            ezk_sip_ua::invite::session::Event::RefreshNeeded(_refresh_needed) => Ok(Some(StateOut::Continue)),
            ezk_sip_ua::invite::session::Event::ReInviteReceived(_re_invite_received) => Ok(Some(StateOut::Continue)),
            ezk_sip_ua::invite::session::Event::Bye(_) => {
                log::info!("[TalkingState] on Bye");
                Ok(Some(StateOut::Event(build_sip_event(sip_event::Event::Bye(sip_event::Bye {})))))
            }
            ezk_sip_ua::invite::session::Event::Terminated => {
                log::info!("[TalkingState] on Terminated");
                Ok(None)
            }
        }
    }
}
