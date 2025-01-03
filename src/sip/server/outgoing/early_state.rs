use ezk_sip_ua::invite::{create_ack, initiator::Early};

use crate::{
    protocol::protobuf::sip_gateway::outgoing_call_data::{
        outgoing_call_event::{self, sip_event},
        OutgoingCallEvent,
    },
    sip::server::outgoing::{build_sip_event, talking_state::TalkingState, State},
    utils::select2,
};

use super::{canceling_state::CancelingState, Ctx, SipOutgoingCallError, StateLogic, StateOut};

#[derive(Debug)]
pub struct EarlyState {
    early: Early,
    cancelled: bool,
}

impl EarlyState {
    pub fn new(early: Early) -> Self {
        Self { early, cancelled: false }
    }
}

impl StateLogic for EarlyState {
    async fn start(&mut self, _ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        Ok(())
    }

    async fn end(&mut self, ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        let mut cancel = ctx.initiator.create_cancel();
        if let Some(auth) = &mut ctx.auth {
            auth.session.authorize_request(&mut cancel.headers);
        }
        log::info!("[EarlyState] end => send cancel");
        ctx.initiator.send_cancel(cancel).await?;
        self.cancelled = true;
        Ok(())
    }

    async fn recv(&mut self, ctx: &mut Ctx) -> Result<Option<StateOut>, SipOutgoingCallError> {
        if self.cancelled {
            return Ok(Some(StateOut::Switch(
                State::Canceling(CancelingState),
                OutgoingCallEvent {
                    event: Some(outgoing_call_event::Event::Cancelled(Default::default())),
                },
            )));
        }

        match select2::or(ctx.initiator.receive(), self.early.receive()).await {
            select2::OrOutput::Left(event) => match event? {
                ezk_sip_ua::invite::initiator::Response::Provisional(_tsx_response) => {
                    unreachable!()
                }
                ezk_sip_ua::invite::initiator::Response::Failure(response) => {
                    // we dont exit here, after that Finished will be called
                    let code = response.line.code.into_u16();
                    log::info!("[EarlyState] on Failure {code}");
                    Ok(Some(StateOut::Event(build_sip_event(sip_event::Event::Failure(sip_event::Failure { code: code as u32 })))))
                }
                ezk_sip_ua::invite::initiator::Response::Early(_early, _tsx_response, _rseq) => {
                    unreachable!()
                }
                ezk_sip_ua::invite::initiator::Response::Session(_session, _tsx_response) => {
                    unreachable!()
                }
                ezk_sip_ua::invite::initiator::Response::Finished => {
                    log::info!("[EarlyState] on Finished");
                    Ok(None)
                }
            },
            select2::OrOutput::Right(event) => match event? {
                ezk_sip_ua::invite::initiator::EarlyResponse::Provisional(response, _rseq) => {
                    let code = response.line.code.into_u16();
                    log::info!("[EarlyState] on Provisional {code}");
                    if !ctx.rtp.answered() && !response.body.is_empty() {
                        ctx.rtp.set_answer(response.body.clone()).await?;
                    }

                    Ok(Some(StateOut::Continue))
                }
                ezk_sip_ua::invite::initiator::EarlyResponse::Success(session, response) => {
                    {
                        let cseq_num = response.base_headers.cseq.cseq;
                        let mut ack_out = create_ack(&session.dialog, cseq_num).await.unwrap();
                        session.endpoint.send_outgoing_request(&mut ack_out).await.unwrap();
                    };

                    let code = response.line.code.into_u16();
                    log::info!("[EarlyState] on Success code: {code} body: {}", String::from_utf8_lossy(&response.body));
                    if !ctx.rtp.answered() && !response.body.is_empty() {
                        ctx.rtp.set_answer(response.body.clone()).await?;
                    }

                    Ok(Some(StateOut::Switch(
                        State::Talking(TalkingState::new(session)),
                        build_sip_event(sip_event::Event::Accepted(sip_event::Accepted { code: code as u32 })),
                    )))
                }
                ezk_sip_ua::invite::initiator::EarlyResponse::Terminated => {
                    log::info!("[EarlyState] on Terminated");
                    Ok(None)
                }
            },
        }
    }
}
