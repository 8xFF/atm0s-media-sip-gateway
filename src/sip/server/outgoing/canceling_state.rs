use ezk_sip_ua::invite::initiator::Response;

use super::{Ctx, SipOutgoingCallError, StateLogic, StateOut};

#[derive(Debug)]
pub struct CancelingState;

impl StateLogic for CancelingState {
    async fn start(&mut self, _ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        Ok(())
    }

    async fn end(&mut self, _ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        Ok(())
    }

    async fn recv(&mut self, ctx: &mut Ctx) -> Result<Option<StateOut>, SipOutgoingCallError> {
        loop {
            if let Response::Finished = ctx.initiator.receive().await? {
                log::info!("[CancelingState] on Finished");
                break Ok(None);
            }
        }
    }
}
