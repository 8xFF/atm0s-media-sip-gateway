use bytesstr::BytesStr;
use ezk_sip_types::header::typed::ContentType;
use ezk_sip_ua::invite::{create_ack, initiator::Response};

use crate::{
    protocol::protobuf::sip_gateway::outgoing_call_data::{
        outgoing_call_event::{self, sip_event},
        OutgoingCallEvent,
    },
    sip::server::outgoing::{build_sip_event, early_state::EarlyState, talking_state::TalkingState, State},
};

use super::{canceling_state::CancelingState, Ctx, SipOutgoingCallError, StateLogic, StateOut};

#[derive(Debug, Default)]
pub struct CallingState {
    auth_failed: bool,
    cancelled: bool,
}

impl StateLogic for CallingState {
    async fn start(&mut self, ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        if ctx.rtp.sdp().is_none() {
            log::info!("[CallingState] creating rtp-sdp");
            ctx.rtp.create_offer().await?;
        }

        let sdp = ctx.rtp.sdp().expect("should have sdp");
        let mut invite = ctx.initiator.create_invite();
        if let Some(ref proxy_uri) = ctx.proxy_uri {
            log::info!("[CallingState] replace uri {:?} with proxy_uri {proxy_uri:?}", invite.line.uri);
            invite.line.uri = proxy_uri.clone();
        }
        invite.body = sdp.clone();
        invite.headers.insert_named(&ContentType(BytesStr::from_static("application/sdp")));
        if let Some(auth) = &mut ctx.auth {
            log::info!("[CallingState] add authorize to headers");
            auth.session.authorize_request(&mut invite.headers);
        }

        log::info!("[CallingState] send invite");
        ctx.initiator.send_invite(invite).await?;
        Ok(())
    }

    async fn end(&mut self, ctx: &mut Ctx) -> Result<(), SipOutgoingCallError> {
        let mut cancel = ctx.initiator.create_cancel();
        if let Some(auth) = &mut ctx.auth {
            auth.session.authorize_request(&mut cancel.headers);
        }
        log::info!("[CallingState] end => send cancel");
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

        match ctx.initiator.receive().await? {
            Response::Provisional(response) => {
                let code = response.line.code.into_u16();
                log::info!("[CallingState] on Provisional {code}");
                Ok(Some(StateOut::Event(build_sip_event(sip_event::Event::Provisional(sip_event::Provisional { code: code as u32 })))))
            }
            Response::Failure(response) => {
                // we dont exit here, after that Finished will be called
                let code = response.line.code.into_u16();

                log::info!("[CallingState] on Failure {code}");
                if (code != 401 && code != 407) || self.auth_failed {
                    return Ok(Some(StateOut::Event(build_sip_event(sip_event::Event::Failure(sip_event::Failure { code: code as u32 })))));
                }

                if code == 401 || code == 407 {
                    self.auth_failed = true;
                }

                if let Some(auth) = &mut ctx.auth {
                    let tsx = ctx.initiator.transaction().expect("should have transaction");
                    let inv = tsx.request();

                    auth.session.handle_authenticate(
                        &response.headers,
                        &auth.credentials,
                        ezk_sip_auth::RequestParts {
                            line: &inv.msg.line,
                            headers: &inv.msg.headers,
                            body: b"",
                        },
                    )?;

                    self.start(ctx).await?;
                    Ok(Some(StateOut::Continue))
                } else {
                    Ok(Some(StateOut::Event(build_sip_event(sip_event::Event::Failure(sip_event::Failure { code: code as u32 })))))
                }
            }
            Response::Early(early, response, _rseq) => {
                let code = response.line.code.into_u16();
                log::info!("[CallingState] switch early with code: {code}");
                Ok(Some(StateOut::Switch(
                    State::Early(EarlyState::new(early)),
                    build_sip_event(sip_event::Event::Early(sip_event::Early { code: code as u32 })),
                )))
            }
            Response::Session(session, response) => {
                let cseq_num = response.base_headers.cseq.cseq;
                let mut ack_out = create_ack(&session.dialog, cseq_num).await.expect("should create ack");
                session.endpoint.send_outgoing_request(&mut ack_out).await?;

                let code = response.line.code.into_u16();
                log::info!("[CallingState] success code: {code} body: {}", String::from_utf8_lossy(&response.body));
                if !ctx.rtp.answered() && !response.body.is_empty() {
                    ctx.rtp.set_answer(response.body.clone()).await?;
                }

                Ok(Some(StateOut::Switch(
                    State::Talking(TalkingState::new(session)),
                    build_sip_event(sip_event::Event::Accepted(sip_event::Accepted { code: code as u32 })),
                )))
            }
            Response::Finished => {
                log::info!("[CallingState] on Finished");
                Ok(None)
            }
        }
    }
}
