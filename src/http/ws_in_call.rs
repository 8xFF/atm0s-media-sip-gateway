use std::{sync::Arc, time::Duration};

use crate::{
    protocol::{
        protobuf::sip_gateway::{
            incoming_call_data::{self, incoming_call_response, IncomingCallEvent, IncomingCallResponse},
            IncomingCallData,
        },
        InternalCallId,
    },
    secure::SecureContext,
    utils::select2::{self, OrOutput},
};

use atm0s_small_p2p::pubsub_service::{PubsubServiceRequester, SubscriberEventOb};
use futures_util::{SinkExt, StreamExt};
use poem::{
    handler,
    web::{
        websocket::{Message as WebsocketMessage, WebSocket},
        Data, Path, Query,
    },
    IntoResponse, Response,
};
use prost::Message;
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::mpsc::unbounded_channel;

const RPC_TIMEOUT_SECONDS: u64 = 2;

#[derive(Clone)]
pub struct WebsocketCallCtx {
    pub secure_ctx: Arc<SecureContext>,
    pub call_pubsub: PubsubServiceRequester,
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    token: String,
}

#[handler]
pub async fn ws_single_call(Path(call_id): Path<String>, Query(query): Query<WsQuery>, ws: WebSocket, data: Data<&WebsocketCallCtx>) -> impl IntoResponse {
    let token = query.token;
    if let Some(token) = data.secure_ctx.decode_call_token(&token) {
        if *token.call_id != call_id {
            return Response::builder().status(StatusCode::BAD_REQUEST).finish();
        }
    } else {
        return Response::builder().status(StatusCode::UNAUTHORIZED).finish();
    }

    let call_id: InternalCallId = call_id.into();
    let mut subscriber = data.call_pubsub.subscriber(call_id.to_pubsub_channel()).await;
    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();
        let (out_tx, mut out_rx) = unbounded_channel();
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            let out = select2::or(select2::or(subscriber.recv_ob::<IncomingCallEvent>(), out_rx.recv()), select2::or(stream.next(), interval.tick())).await;
            match out {
                OrOutput::Left(OrOutput::Left(Ok(event))) => match event {
                    SubscriberEventOb::PeerJoined(peer_src) => {
                        log::info!("[WsCall {call_id}] publisher {peer_src:?} joined");
                    }
                    SubscriberEventOb::PeerLeaved(peer_src) => {
                        log::info!("[WsCall {call_id}] publisher {peer_src:?} leaved");
                    }
                    SubscriberEventOb::Publish(msg) => {
                        log::info!("[WsCall {call_id}] got publisher message {msg:?}");
                        let _ = out_tx.send(IncomingCallData {
                            data: Some(incoming_call_data::Data::Event(msg)),
                        });
                    }
                    _ => {
                        log::warn!("[WsCall {call_id}] unhandled pubsub event {event:?}");
                    }
                },
                OrOutput::Left(OrOutput::Left(_)) => {
                    break;
                }
                OrOutput::Left(OrOutput::Right(event)) => match event {
                    Some(msg) => {
                        let data = msg.encode_to_vec();
                        log::info!("[WsCall {call_id}] emit data {msg:?}");
                        if let Err(e) = sink.send(WebsocketMessage::Binary(data)).await {
                            log::error!("[WsCall {call_id}] send data error {e:?}");
                            break;
                        }
                    }
                    None => break,
                },
                OrOutput::Right(OrOutput::Left(Some(Ok(message)))) => {
                    if let WebsocketMessage::Binary(msg) = message {
                        match IncomingCallData::decode(msg.as_slice()) {
                            Ok(data) => match data.data {
                                Some(incoming_call_data::Data::Request(req)) => {
                                    log::info!("[WsCall {call_id}] on incoming req {} {:?}", req.req_id, req.action);
                                    let subscriber = subscriber.requester().clone();
                                    let call_id = call_id.clone();
                                    let out_tx = out_tx.clone();
                                    tokio::spawn(async move {
                                        let action = if let Some(action) = req.action {
                                            action
                                        } else {
                                            return;
                                        };
                                        let res = subscriber
                                            .feedback_rpc_ob::<_, incoming_call_response::Response>("action", &action, Duration::from_secs(RPC_TIMEOUT_SECONDS))
                                            .await
                                            .map_err(|e| e.to_string());

                                        let response = match res {
                                            Ok(res) => res,
                                            Err(message) => incoming_call_response::Response::Error(incoming_call_response::Error { message }),
                                        };
                                        log::info!("[WsCall {call_id}] response incoming req {} {response:?}", req.req_id);
                                        let _ = out_tx.send(IncomingCallData {
                                            data: Some(incoming_call_data::Data::Response(IncomingCallResponse {
                                                req_id: req.req_id,
                                                response: Some(response),
                                            })),
                                        });
                                    });
                                }
                                _ => {
                                    log::error!("[WsCall {call_id}] incoming req unsupported type {data:?}");
                                }
                            },
                            Err(err) => {
                                log::error!("[WsCall {call_id}] parse incoming req error {err:?}");
                            }
                        }
                    }
                    log::info!("[WsCall {call_id}] received data");
                }
                OrOutput::Right(OrOutput::Left(Some(Err(e)))) => {
                    log::error!("[WsCall {call_id}] socket error {e:?}");
                }
                OrOutput::Right(OrOutput::Left(None)) => {
                    log::info!("[WsCall {call_id}] socket closed");
                    break;
                }
                OrOutput::Right(OrOutput::Right(_)) => {
                    log::info!("[WsCall {call_id}] interval tick");
                    if let Err(e) = sink.send(WebsocketMessage::Ping(vec![])).await {
                        log::error!("[WsCall {call_id}] send data error {e:?}");
                        break;
                    }
                }
            }
        }
    })
    .into_response()
}
