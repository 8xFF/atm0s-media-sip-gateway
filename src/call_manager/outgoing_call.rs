use std::collections::HashMap;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::{
    futures::select2,
    hook::HttpHookSender,
    protocol::{InternalCallId, OutgoingCallEvent, OutgoingCallSipEvent},
    sip::{SipOutgoingCall, SipOutgoingCallError, SipOutgoingCallOut},
};

use super::{EmitterId, EventEmitter};

pub struct OutgoingCall<EM> {
    control_tx: UnboundedSender<CallControl<EM>>,
}

impl<EM: EventEmitter> OutgoingCall<EM> {
    pub fn new(sip: SipOutgoingCall, destroy_tx: UnboundedSender<InternalCallId>, hook: HttpHookSender) -> Self {
        let (control_tx, control_rx) = unbounded_channel();
        tokio::spawn(async move { run_call_loop(sip, control_rx, destroy_tx, hook).await });

        Self { control_tx }
    }

    pub fn add_emitter(&mut self, emitter: EM) {
        if let Err(e) = self.control_tx.send(CallControl::Sub(emitter)) {
            log::error!("[OutgoingCall] send Sub control error {e:?}");
        }
    }

    pub fn del_emitter(&mut self, emitter: EmitterId) {
        if let Err(e) = self.control_tx.send(CallControl::Unsub(emitter)) {
            log::error!("[OutgoingCall] send Unsub control error {e:?}");
        }
    }

    pub fn end(&mut self) {
        if let Err(e) = self.control_tx.send(CallControl::End) {
            log::error!("[OutgoingCall] send End control error {e:?}");
        }
    }
}

enum CallControl<EM> {
    Sub(EM),
    Unsub(EmitterId),
    End,
}

async fn run_call_loop<EM: EventEmitter>(mut call: SipOutgoingCall, mut control_rx: UnboundedReceiver<CallControl<EM>>, destroy_tx: UnboundedSender<InternalCallId>, hook: HttpHookSender) {
    let call_id = call.call_id();
    let mut emitters: HashMap<EmitterId, EM> = HashMap::new();

    log::info!("[OutgoingCall] call starting");

    if let Err(e) = call.start().await {
        log::error!("[OutgoingCall] call start error {e:?}");
        destroy_tx.send(call_id).expect("should send destroy request to main loop");
        return;
    }

    log::info!("[OutgoingCall] call started");

    loop {
        let out = select2::or(call.recv(), control_rx.recv()).await;
        match out {
            select2::OrOutput::Left(Ok(Some(out))) => match out {
                SipOutgoingCallOut::Event(event) => {
                    let value = serde_json::to_value(&event).expect("should convert to json");
                    for emitter in emitters.values_mut() {
                        emitter.fire(value.clone().into());
                    }
                    hook.send(&event);
                }
                SipOutgoingCallOut::Continue => {}
            },
            select2::OrOutput::Left(Ok(None)) => {
                log::info!("[OutgoingCall] call end");
                break;
            }
            select2::OrOutput::Left(Err(e)) => {
                log::error!("[OutgoingCall] call error {e:?}");
                let event = if let SipOutgoingCallError::Sip(code) = &e {
                    OutgoingCallEvent::Sip(OutgoingCallSipEvent::Failure { code: *code })
                } else {
                    OutgoingCallEvent::Error { message: e.to_string() }
                };

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
                            log::info!("[OutgoingCall] all sub disconnected => end call");
                            if let Err(e) = call.end().await {
                                log::error!("[OutgoingCall] end call error {e:?}");
                            }
                            break;
                        }
                    }
                }
                CallControl::End => {
                    log::info!("[OutgoingCall] received end request");
                    if let Err(e) = call.end().await {
                        log::error!("[OutgoingCall] end call error {e:?}");
                    }
                    break;
                }
            },
            select2::OrOutput::Right(None) => {
                break;
            }
        }
    }

    log::info!("[OutgoingCall] call destroyed");
    let event = OutgoingCallEvent::Destroyed;
    let value = serde_json::to_value(&event).expect("should convert to json");
    for emitter in emitters.values_mut() {
        emitter.fire(value.clone().into());
    }
    hook.send(&event);
    destroy_tx.send(call_id).expect("should send destroy request to main loop");
}
