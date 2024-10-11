use std::sync::Arc;

use bytes::Bytes;
use bytesstr::BytesStr;
use ezk_sip_types::{header::typed::ContentType, Code};
use ezk_sip_ua::invite::acceptor::Acceptor;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Notify,
};

use crate::{
    error::PrintErrorSimple,
    futures::select2,
    protocol::{IncomingCallEvent, IncomingCallSipEvent, StreamingInfo},
    sip::{media::MediaRtpEngineAnswer, MediaApi},
};

use super::{talking_state::TalkingState, Ctx, SipIncomingCallError, State, StateLogic, StateOut};

pub struct WaitState {
    cancelled: Arc<Notify>,
    acceptor: Option<Acceptor>,
    offer_sdp: Bytes,
    tx: UnboundedSender<Option<StateOut>>,
    rx: UnboundedReceiver<Option<StateOut>>,
}

impl WaitState {
    pub fn new(acceptor: Acceptor, offer_sdp: Bytes, cancelled: Arc<Notify>) -> Self {
        let (tx, rx) = unbounded_channel();
        Self {
            cancelled,
            acceptor: Some(acceptor),
            offer_sdp,
            tx,
            rx,
        }
    }
}

impl StateLogic for WaitState {
    async fn send_trying(&mut self, _ctx: &mut Ctx) -> Result<(), SipIncomingCallError> {
        let acceptor = self.acceptor.as_mut().expect("should have acceptor when start called");
        let response = acceptor.create_response(Code::TRYING, None).await?;
        acceptor.respond_provisional(response).await?;
        Ok(())
    }

    async fn send_ringing(&mut self, _ctx: &mut Ctx) -> Result<(), SipIncomingCallError> {
        let acceptor = self.acceptor.as_mut().expect("should have acceptor when ring called");
        let response = acceptor.create_response(Code::RINGING, None).await?;
        acceptor.respond_provisional(response).await?;
        Ok(())
    }

    async fn accept(&mut self, _ctx: &mut Ctx, api: MediaApi, stream: StreamingInfo) -> Result<(), SipIncomingCallError> {
        log::info!("[IncomingCall/WaitState] accept");
        let mut response = self.acceptor.as_mut().expect("should have acceptor when start called").create_response(Code::OK, None).await?;

        let mut rtp = MediaRtpEngineAnswer::new(api, self.offer_sdp.clone());
        let answer_sdp = rtp.create_answer(&stream).await?;

        response.msg.body = answer_sdp;
        response.msg.headers.insert_named(&ContentType(BytesStr::from_static("application/sdp")));

        let (session, _) = self.acceptor.take().expect("should have acceptor").respond_success(response).await?;
        self.tx
            .send(Some(StateOut::Switch(State::Talking(TalkingState::new(session, rtp)), IncomingCallEvent::Accepted)))
            .expect("should send to parent");
        Ok(())
    }

    async fn end(&mut self, _ctx: &mut Ctx) -> Result<(), SipIncomingCallError> {
        log::info!("[IncomingCall/WaitState] end");
        let acceptor = self.acceptor.take().expect("should have acceptor when start called");
        let response = acceptor.create_response(Code::BUSY_HERE, None).await?;
        acceptor.respond_failure(response).await?;
        self.tx.send(None).expect("should send to parent");
        Ok(())
    }

    fn kill_because_validate_failed(mut self, _ctx: &mut Ctx) {
        let acceptor = self.acceptor.take().expect("should have acceptor when kill called");
        tokio::spawn(async move {
            reject_call(acceptor, Code::NOT_ACCEPTABLE).await.print_error("[SipIncoming] reject call");
        });
    }

    async fn recv(&mut self, _ctx: &mut Ctx) -> Result<Option<StateOut>, SipIncomingCallError> {
        let wait_cancelled = self.cancelled.notified();
        loop {
            let out = select2::or(self.rx.recv(), wait_cancelled).await;
            match out {
                select2::OrOutput::Left(event) => return Ok(event.expect("")),
                select2::OrOutput::Right(_) => {
                    self.tx.send(None).expect("should send to parent");
                    return Ok(Some(StateOut::Event(IncomingCallEvent::Sip(IncomingCallSipEvent::Cancelled))));
                }
            }
        }
    }
}

async fn reject_call(acceptor: Acceptor, code: Code) -> anyhow::Result<()> {
    let response = acceptor.create_response(code, None).await?;
    acceptor.respond_failure(response).await?;
    Ok(())
}
