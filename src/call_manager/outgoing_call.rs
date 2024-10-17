use std::collections::HashSet;

use atm0s_small_p2p::pubsub_service::{PublisherEventOb, PubsubServiceRequester};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    error::PrintErrorSimple,
    hook::HttpHookSender,
    protocol::{
        protobuf::sip_gateway::outgoing_call_data::{outgoing_call_event, outgoing_call_request, outgoing_call_response, OutgoingCallEvent},
        InternalCallId,
    },
    sip::{SipOutgoingCall, SipOutgoingCallOut},
    utils::select2,
};

pub struct OutgoingCall {}

impl OutgoingCall {
    pub fn new(sip: SipOutgoingCall, destroy_tx: UnboundedSender<InternalCallId>, hook: HttpHookSender, call_pubsub: PubsubServiceRequester) -> Self {
        tokio::spawn(async move { run_call_loop(sip, destroy_tx, hook, call_pubsub).await });

        Self {}
    }
}

async fn run_call_loop(mut call: SipOutgoingCall, destroy_tx: UnboundedSender<InternalCallId>, hook: HttpHookSender, call_pubsub: PubsubServiceRequester) {
    let call_id = call.call_id();
    let channel_id = call_id.to_pubsub_channel();
    let mut subscribers = HashSet::new();
    let mut publisher = call_pubsub.publisher(channel_id).await;

    log::info!("[OutgoingCall] call starting");

    if let Err(e) = call.start().await {
        log::error!("[OutgoingCall] call start error {e:?}");
        destroy_tx.send(call_id).expect("should send destroy request to main loop");
        return;
    }

    log::info!("[OutgoingCall] call started");

    loop {
        let out = select2::or(call.recv(), publisher.recv_ob::<outgoing_call_request::Action>()).await;
        match out {
            select2::OrOutput::Left(Ok(Some(out))) => match out {
                SipOutgoingCallOut::Event(event) => {
                    hook.send(&event);
                    publisher.requester().publish_ob(&event).await.print_error("[OutgoingCall] send event");
                }
                SipOutgoingCallOut::Continue => {}
            },
            select2::OrOutput::Left(Ok(None)) => {
                log::info!("[OutgoingCall] call end");
                break;
            }
            select2::OrOutput::Left(Err(e)) => {
                log::error!("[OutgoingCall] call error {e:?}");
                let event = OutgoingCallEvent {
                    event: Some(outgoing_call_event::Event::Err(outgoing_call_event::Error { message: e.to_string() })),
                };
                publisher.requester().publish_ob(&event).await.print_error("[OutgoingCall] send event");
                hook.send(&event);
                break;
            }
            select2::OrOutput::Right(Ok(control)) => match control {
                PublisherEventOb::PeerJoined(peer_src) => {
                    subscribers.insert(peer_src);
                }
                PublisherEventOb::PeerLeaved(peer_src) => {
                    if subscribers.remove(&peer_src) && subscribers.is_empty() {
                        log::info!("[OutgoingCall] all sub disconnected => end call");
                        if let Err(e) = call.end().await {
                            log::error!("[OutgoingCall] end call error {e:?}");
                        }
                    }
                }
                PublisherEventOb::FeedbackRpc(action, rpc_id, _method, peer_src) | PublisherEventOb::GuestFeedbackRpc(action, rpc_id, _method, peer_src) => match action {
                    outgoing_call_request::Action::End(_end) => {
                        log::info!("[OutgoingCall] call {call_id} received end request");
                        let res = if let Err(e) = call.end().await {
                            log::error!("[OutgoingCall] call {call_id} end error {e:?}");
                            outgoing_call_response::Response::Error(outgoing_call_response::Error { message: e.to_string() })
                        } else {
                            outgoing_call_response::Response::End(Default::default())
                        };
                        publisher.requester().answer_feedback_rpc_ob(rpc_id, peer_src, &res).await.print_error("[IncomingCall] answer rpc");
                    }
                },
                _ => {}
            },
            select2::OrOutput::Right(Err(_e)) => {
                break;
            }
        }
    }

    log::info!("[OutgoingCall] call destroyed");
    let event = OutgoingCallEvent {
        event: Some(outgoing_call_event::Event::Ended(Default::default())),
    };
    publisher.requester().publish_ob(&event).await.print_error("[IncomingCall] publish event");
    hook.send(&event);
    destroy_tx.send(call_id).expect("should send destroy request to main loop");
}
