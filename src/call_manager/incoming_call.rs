use std::collections::HashSet;

use anyhow::anyhow;
use atm0s_small_p2p::pubsub_service::{PublisherEventOb, PubsubServiceRequester};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    error::PrintErrorSimple,
    hook::HttpHookSender,
    protocol::{
        protobuf::sip_gateway::incoming_call_data::{incoming_call_event, incoming_call_request, incoming_call_response, IncomingCallEvent},
        HookIncomingCallRequest, HookIncomingCallResponse, IncomingCallAction, InternalCallId, StreamingInfo,
    },
    sip::{MediaApi, SipIncomingCall, SipIncomingCallOut},
    utils::select2,
};

pub struct IncomingCall {}

impl IncomingCall {
    pub fn new(api: MediaApi, sip: SipIncomingCall, call_token: String, destroy_tx: UnboundedSender<InternalCallId>, hook: HttpHookSender, call_pubsub: PubsubServiceRequester) -> Self {
        tokio::spawn(async move {
            let call_id = sip.call_id();
            if let Err(e) = run_call_loop(api, sip, call_token, hook, call_pubsub).await {
                log::error!("[IncomingCall] call {call_id} error {e:?}");
            }
            destroy_tx.send(call_id).expect("should send destroy request to main loop");
        });

        Self {}
    }
}

async fn run_call_loop(api: MediaApi, mut call: SipIncomingCall, call_token: String, hook: HttpHookSender, call_pubsub: PubsubServiceRequester) -> anyhow::Result<()> {
    let call_id = call.call_id();
    let from = call.from().to_owned();
    let to = call.to().to_owned();

    let channel_id = call_id.to_pubsub_channel();
    let mut subscribers = HashSet::new();
    let call_ws = format!("/call/incoming/{call_id}?token={call_token}");
    log::info!("[IncomingCall] call {call_id} start, ws: {call_ws}, sending hook ...");

    // we send trying first
    call.send_trying().await?;
    let mut publisher = call_pubsub.publisher(channel_id).await;

    // feedback hook for info
    let res: HookIncomingCallResponse = hook
        .request(&HookIncomingCallRequest {
            call_id: call_id.clone(),
            call_token,
            call_ws,
            from,
            to,
        })
        .await?;

    log::info!("[IncomingCall] call {call_id} got hook action {:?}", res.action);

    match res.action {
        IncomingCallAction::Ring => call.send_ringing().await?,
        IncomingCallAction::Accept => {
            let stream = res.stream.ok_or(anyhow!("missing stream in accept action"))?;
            call.accept(api.clone(), stream).await?;
        }
        IncomingCallAction::End => {
            call.end().await.print_error("[IncomingCall] end call from hook response");
            return Ok(());
        }
    };

    log::info!("[IncomingCall] call {call_id} started loop");

    loop {
        let out = select2::or(call.recv(), publisher.recv_ob::<incoming_call_request::Action>()).await;
        match out {
            select2::OrOutput::Left(Ok(Some(out))) => match out {
                SipIncomingCallOut::Event(event) => {
                    publisher.requester().publish_ob(&event).await.print_error("[IncomingCall] publish event");
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
                let event = IncomingCallEvent {
                    event: Some(incoming_call_event::Event::Err(incoming_call_event::Error { message: e.to_string() })),
                };
                publisher.requester().publish_ob(&event).await.print_error("[IncomingCall] publish event");
                hook.send(&event);
                break;
            }
            select2::OrOutput::Right(Ok(control)) => match control {
                PublisherEventOb::PeerJoined(peer_src) => {
                    subscribers.insert(peer_src);
                }
                PublisherEventOb::PeerLeaved(peer_src) => {
                    if subscribers.remove(&peer_src) && subscribers.is_empty() {
                        log::info!("[IncomingCall] call {call_id} all subs disconnected => end call");
                        if let Err(e) = call.end().await {
                            log::error!("[IncomingCall] call {call_id} end error {e:?}");
                        }
                        break;
                    }
                }
                PublisherEventOb::GuestFeedbackRpc(action, rpc_id, method, peer_src) => {
                    log::info!("[IncomingCall] on rpc {method} from {peer_src:?} with payload: {action:?}");
                    let res = match action {
                        incoming_call_request::Action::Ring(_ring) => {
                            if let Err(e) = call.send_trying().await {
                                incoming_call_response::Response::Error(incoming_call_response::Error { message: e.to_string() })
                            } else {
                                incoming_call_response::Response::Ring(Default::default())
                            }
                        }
                        incoming_call_request::Action::Accept(accept) => {
                            let stream = StreamingInfo {
                                room: accept.room,
                                peer: accept.peer,
                                record: accept.record,
                            };
                            if let Err(e) = call.accept(api.clone(), stream).await {
                                incoming_call_response::Response::Error(incoming_call_response::Error { message: e.to_string() })
                            } else {
                                incoming_call_response::Response::Accept(Default::default())
                            }
                        }
                        incoming_call_request::Action::End(_end) => {
                            log::info!("[IncomingCall] call {call_id} received end request");
                            if let Err(e) = call.end().await {
                                log::error!("[IncomingCall] call {call_id} end error {e:?}");
                                incoming_call_response::Response::Error(incoming_call_response::Error { message: e.to_string() })
                            } else {
                                incoming_call_response::Response::End(Default::default())
                            }
                        }
                    };
                    publisher.requester().answer_feedback_rpc_ob(rpc_id, peer_src, &res).await.print_error("[IncomingCall] answer rpc");
                }
                _ => {}
            },
            select2::OrOutput::Right(Err(_e)) => {
                break;
            }
        }
    }

    log::info!("[IncomingCall] call {call_id} destroyed");
    let event = IncomingCallEvent {
        event: Some(incoming_call_event::Event::Ended(Default::default())),
    };
    publisher.requester().publish_ob(&event).await.print_error("[IncomingCall] publish event");
    hook.send(&event);
    Ok(())
}
