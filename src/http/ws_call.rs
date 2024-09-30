use std::sync::Arc;

use crate::{
    call_manager::{EmitterId, EventEmitter},
    futures::select2::{self, OrOutput},
    protocol::{InternalCallId, WsActionResponse, WsMessage},
    secure::SecureContext,
};

use super::HttpCommand;
use futures_util::{SinkExt, StreamExt};
use poem::{
    handler,
    web::{
        websocket::{Message, WebSocket},
        Data, Path, Query,
    },
    IntoResponse, Response,
};
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::{
    mpsc::{unbounded_channel, Sender, UnboundedSender},
    oneshot,
};

#[derive(Clone)]
pub struct WebsocketCtx {
    pub secure_ctx: Arc<SecureContext>,
    pub cmd_tx: Sender<HttpCommand>,
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    token: String,
}

#[handler]
pub fn ws_single_call(Path(call_id): Path<String>, Query(query): Query<WsQuery>, ws: WebSocket, data: Data<&WebsocketCtx>) -> impl IntoResponse {
    let token = query.token;
    if let Some(token) = data.secure_ctx.decode_token(&token) {
        if *token.call_id != call_id {
            return Response::builder().status(StatusCode::BAD_REQUEST).finish();
        }
    } else {
        return Response::builder().status(StatusCode::UNAUTHORIZED).finish();
    }

    let cmd_tx = data.cmd_tx.clone();
    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();
        let emitter_id = EmitterId::rand();
        let call_id: InternalCallId = call_id.into();
        let (out_tx, mut out_rx) = unbounded_channel();
        let _out_tx = out_tx.clone(); //we need to store it for avoiding ws error when call dropped
        let emitter = WebsocketEventEmitter { emitter_id, out_tx: out_tx.clone() };

        let (tx, rx) = oneshot::channel();
        if let Err(e) = cmd_tx.send(HttpCommand::SubscribeCall(call_id.clone(), emitter, tx)).await {
            log::error!("[WsCall {call_id}/{emitter_id}] send sub_cmd error {e:?}");
            return;
        }

        match rx.await {
            Ok(res) => match res {
                Ok(_) => {}
                Err(err) => {
                    log::error!("[WsCall {call_id}/{emitter_id}] sub_cmd got error {err:?}");
                    return;
                }
            },
            Err(err) => {
                log::error!("[WsCall {call_id}/{emitter_id}] send sub_cmd error {err:?}");
                return;
            }
        }

        loop {
            let out = select2::or(out_rx.recv(), stream.next()).await;
            match out {
                OrOutput::Left(Some(event)) => {
                    let msg = serde_json::to_string(&event).expect("should convert to json string");
                    if let Err(e) = sink.send(Message::Text(msg)).await {
                        log::error!("[WsCall {call_id}/{emitter_id}] send data error {e:?}");
                        break;
                    }
                }
                OrOutput::Left(_) => {
                    break;
                }
                OrOutput::Right(Some(Ok(message))) => {
                    if let Message::Text(msg) = message {
                        match serde_json::from_str::<WsMessage>(&msg) {
                            Ok(WsMessage::Request(req)) => {
                                log::error!("[WsCall {call_id}/{emitter_id}] on incoming req {} {:?}", req.request_id, req.request);
                                let (tx, rx) = oneshot::channel();
                                let out_tx = out_tx.clone();
                                let cmd_tx = cmd_tx.clone();
                                let call_id = call_id.clone();
                                tokio::spawn(async move {
                                    let res = if let Err(err) = cmd_tx.send(HttpCommand::ActionCall(call_id.clone(), req.request, tx)).await {
                                        log::error!("[WsCall {call_id}/{emitter_id}] send ws action error {err:?}");
                                        Err(format!("server error: {err}"))
                                    } else {
                                        match rx.await {
                                            Ok(res) => res.map_err(|e| e.to_string()),
                                            Err(err) => Err(format!("server error: {err}")),
                                        }
                                    };

                                    let res = match res {
                                        Ok(res) => WsActionResponse {
                                            request_id: Some(req.request_id),
                                            success: true,
                                            response: Some(res),
                                            ..Default::default()
                                        },
                                        Err(err) => WsActionResponse {
                                            request_id: Some(req.request_id),
                                            success: false,
                                            message: Some(err),
                                            ..Default::default()
                                        },
                                    };
                                    log::info!("[WsCall {call_id}/{emitter_id}] response incoming req {} {res:?}", req.request_id);
                                    let _ = out_tx.send(WsMessage::Response(res));
                                });
                            }
                            Ok(_) => {
                                log::error!("[WsCall {call_id}/{emitter_id}] parse incoming req {msg} unsupported type");
                                let _ = out_tx.send(WsMessage::Response(WsActionResponse {
                                    request_id: None,
                                    success: false,
                                    message: Some("unsupported type".to_string()),
                                    response: None,
                                }));
                            }
                            Err(err) => {
                                log::error!("[WsCall {call_id}/{emitter_id}] parse incoming req {msg} error {err}");
                                let _ = out_tx.send(WsMessage::Response(WsActionResponse {
                                    request_id: None,
                                    success: false,
                                    message: Some(format!("message parse failured: {err}")),
                                    response: None,
                                }));
                            }
                        }
                    }
                    log::info!("[WsCall {call_id}/{emitter_id}] received data");
                }
                OrOutput::Right(_) => {
                    log::info!("[WsCall {call_id}/{emitter_id}] socket closed");
                    break;
                }
            }
        }

        let (tx, rx) = oneshot::channel();
        if let Err(e) = cmd_tx.send(HttpCommand::UnsubscribeCall(call_id.clone(), emitter_id, tx)).await {
            log::error!("[WsCall {call_id}/{emitter_id}] send sub_cmd error {e:?}");
            return;
        }

        match rx.await {
            Ok(res) => match res {
                Ok(_) => {}
                Err(err) => {
                    log::error!("[WsCall {call_id}/{emitter_id}] sub_cmd got error {err:?}");
                    return;
                }
            },
            Err(err) => {
                log::error!("[WsCall {call_id}/{emitter_id}] send sub_cmd error {err:?}");
                return;
            }
        }
    })
    .into_response()
}

pub struct WebsocketEventEmitter {
    emitter_id: EmitterId,
    out_tx: UnboundedSender<WsMessage>,
}

impl EventEmitter for WebsocketEventEmitter {
    fn emitter_id(&self) -> EmitterId {
        self.emitter_id
    }

    fn fire(&mut self, msg: WsMessage) {
        if let Err(e) = self.out_tx.send(msg) {
            log::error!("[WebsocketEventEmitter] send event error {e:?}");
        }
    }
}
