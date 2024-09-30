use ezk_sip_ua::invite::session::Session;

use crate::protocol::{OutgoingCallEvent, OutgoingCallSipEvent};

use super::{Ctx, SipOutgoingCallError, StateLogic, StateOut};

pub struct TalkingState {
    session: Session,
}

impl TalkingState {
    pub fn new(session: Session) -> Self {
        Self { session }
    }
}

impl StateLogic for TalkingState {
    async fn start(&mut self, _ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        Ok(())
    }
    async fn end(&mut self, _ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        self.session.terminate().await?;
        Ok(())
    }
    async fn recv(&mut self, _ctx: &mut Ctx) -> Result<Option<StateOut>, SipOutgoingCallError> {
        match self.session.drive().await? {
            ezk_sip_ua::invite::session::Event::RefreshNeeded(_refresh_needed) => Ok(Some(StateOut::Continue)),
            ezk_sip_ua::invite::session::Event::ReInviteReceived(_re_invite_received) => Ok(Some(StateOut::Continue)),
            ezk_sip_ua::invite::session::Event::Bye(_) => {
                log::info!("[TalkingState] on Bye");
                Ok(Some(StateOut::Event(OutgoingCallEvent::Sip(OutgoingCallSipEvent::Bye {}))))
            }
            ezk_sip_ua::invite::session::Event::Terminated => {
                log::info!("[TalkingState] on Terminated");
                Ok(None)
            }
        }
    }
}
