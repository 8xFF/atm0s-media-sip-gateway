use std::collections::HashMap;

use anyhow::anyhow;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{
    error::PrintErrorDetails,
    hook::HttpHookSender,
    protocol::{CallAction, CallActionRequest, CallActionResponse, HookIncomingCallRequest, HookIncomingCallResponse, IncomingCallEvent, InternalCallId},
    sip::{MediaApi, SipIncomingCall, SipIncomingCallOut},
    utils::http_to_ws,
    utils::select2,
};

use super::{EmitterId, EventEmitter};

pub struct IncomingCall<EM> {
    control_tx: UnboundedSender<CallControl<EM>>,
}

impl<EM: EventEmitter> IncomingCall<EM> {
    pub fn new(http_public: &str, api: MediaApi, sip: SipIncomingCall, call_token: String, destroy_tx: UnboundedSender<InternalCallId>, hook: HttpHookSender) -> Self {
        let (control_tx, control_rx) = unbounded_channel();
        let http_public = http_public.to_owned();
        tokio::spawn(async move {
            let call_id = sip.call_id();
            if let Err(e) = run_call_loop(&http_public, api, sip, call_token, control_rx, hook).await {
                log::error!("[IncomingCall] call {call_id} error {e:?}");
            }
            destroy_tx.send(call_id).expect("should send destroy request to main loop");
        });

        Self { control_tx }
    }

    pub fn add_emitter(&mut self, emitter: EM) {
        if let Err(e) = self.control_tx.send(CallControl::Sub(emitter)) {
            log::error!("[IncomingCall] send Sub control error {e:?}");
        }
    }

    pub fn del_emitter(&mut self, emitter: EmitterId) {
        if let Err(e) = self.control_tx.send(CallControl::Unsub(emitter)) {
            log::error!("[IncomingCall] send Unsub control error {e:?}");
        }
    }

    pub fn do_action(&mut self, action: CallActionRequest, tx: oneshot::Sender<anyhow::Result<CallActionResponse>>) {
        if let Err(e) = self.control_tx.send(CallControl::Action(action, tx)) {
            log::error!("[IncomingCall] send Unsub control error {e:?}");
        }
    }

    pub fn end(&mut self) {
        if let Err(e) = self.control_tx.send(CallControl::End) {
            log::error!("[IncomingCall] send End control error {e:?}");
        }
    }
}

enum CallControl<EM> {
    Sub(EM),
    Unsub(EmitterId),
    Action(CallActionRequest, oneshot::Sender<anyhow::Result<CallActionResponse>>),
    End,
}

async fn run_call_loop<EM: EventEmitter>(
    http_public: &str,
    api: MediaApi,
    mut call: SipIncomingCall,
    call_token: String,
    mut control_rx: UnboundedReceiver<CallControl<EM>>,
    hook: HttpHookSender,
) -> anyhow::Result<()> {
    let call_id = call.call_id();
    let from = call.from().to_owned();
    let to = call.to().to_owned();

    let mut emitters: HashMap<EmitterId, EM> = HashMap::new();
    let call_ws = format!("{}/ws/call/{call_id}?token={call_token}", http_to_ws(http_public));
    log::info!("[IncomingCall] call {call_id} start, ws: {call_ws}, sending hook ...");

    // we send trying first
    call.send_trying().await?;

    // feedback hook for info
    let res: HookIncomingCallResponse = hook
        .request(&HookIncomingCallRequest {
            gateway: http_public.to_owned(),
            call_id: call_id.clone(),
            call_token,
            call_ws,
            from,
            to,
        })
        .await?;

    log::info!("[IncomingCall] call {call_id} got hook action {:?}", res.action);

    match res.action {
        CallAction::Trying => {}
        CallAction::Ring => call.send_ringing().await?,
        CallAction::Reject => {
            call.kill_because_validate_failed();
            return Ok(());
        }
        CallAction::Accept => {
            let stream = res.stream.ok_or(anyhow!("missing stream in accept action"))?;
            call.accept(api.clone(), stream).await?;
        }
    };

    log::info!("[IncomingCall] call {call_id} started loop");

    loop {
        let out = select2::or(call.recv(), control_rx.recv()).await;
        match out {
            select2::OrOutput::Left(Ok(Some(out))) => match out {
                SipIncomingCallOut::Event(event) => {
                    let value = serde_json::to_value(&event).expect("should convert to json");
                    for emitter in emitters.values_mut() {
                        emitter.fire(value.clone().into());
                    }
                    hook.send(&event);
                }
                SipIncomingCallOut::Continue => {}
            },
            select2::OrOutput::Left(Ok(None)) => {
                log::info!("[IncomingCall] call {call_id} end");
                break;
            }
            select2::OrOutput::Left(Err(e)) => {
                log::error!("[IncomingCall] call {call_id} error {e:?}");
                let event = IncomingCallEvent::Error { message: e.to_string() };
                let value = serde_json::to_value(&event).expect("should convert to json");
                for emitter in emitters.values_mut() {
                    emitter.fire(value.clone().into());
                }
                hook.send(&event);
                break;
            }
            select2::OrOutput::Right(Some(control)) => match control {
                CallControl::Sub(emitter) => {
                    emitters.insert(emitter.emitter_id(), emitter);
                }
                CallControl::Unsub(emitter_id) => {
                    if emitters.remove(&emitter_id).is_some() {
                        if emitters.is_empty() {
                            log::info!("[IncomingCall] call {call_id} all subs disconnected => end call");
                            if let Err(e) = call.end().await {
                                log::error!("[IncomingCall] call {call_id} end error {e:?}");
                            }
                            break;
                        }
                    }
                }
                CallControl::End => {
                    log::info!("[IncomingCall] call {call_id} received end request");
                    if let Err(e) = call.end().await {
                        log::error!("[IncomingCall] call {call_id} end error {e:?}");
                    }
                    break;
                }
                CallControl::Action(action, tx) => {
                    let res = match action.action {
                        CallAction::Trying => call.send_trying().await.map_err(|e| e.into()),
                        CallAction::Ring => call.send_ringing().await.map_err(|e| e.into()),
                        CallAction::Reject => call.end().await.map_err(|e| e.into()),
                        CallAction::Accept => {
                            if let Some(stream) = action.stream {
                                call.accept(api.clone(), stream).await.map_err(|e| e.into())
                            } else {
                                Err(anyhow!("missing stream in accept action"))
                            }
                        }
                    };
                    tx.send(res.map(|_| CallActionResponse {})).print_error_detail("[IncomingCall] send action res");
                }
            },
            select2::OrOutput::Right(None) => {
                break;
            }
        }
    }

    log::info!("[IncomingCall] call {call_id} destroyed");
    let event = IncomingCallEvent::Destroyed;
    let value = serde_json::to_value(&event).expect("should convert to json");
    for emitter in emitters.values_mut() {
        emitter.fire(value.clone().into());
    }
    hook.send(&event);
    Ok(())
}
