use std::sync::Arc;

use crate::{
    call_manager::{EmitterId, EventEmitter},
    protocol::WsMessage,
    secure::SecureContext,
};

use futures_util::StreamExt;
use poem::{
    handler,
    web::{websocket::WebSocket, Data, Query},
    IntoResponse, Response,
};
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct WebsocketNotifyCtx {
    pub secure_ctx: Arc<SecureContext>,
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    token: String,
}

#[handler]
pub fn ws_single_notify(Query(query): Query<WsQuery>, ws: WebSocket, data: Data<&WebsocketNotifyCtx>) -> impl IntoResponse {
    let token = query.token;
    let _token = if let Some(token) = data.secure_ctx.decode_call_token(&token) {
        token
    } else {
        return Response::builder().status(StatusCode::UNAUTHORIZED).finish();
    };

    ws.on_upgrade(move |socket| async move {
        let (mut _sink, mut _stream) = socket.split();
    })
    .into_response()
}

pub struct WebsocketNotifyEventEmitter {
    emitter_id: EmitterId,
    out_tx: UnboundedSender<WsMessage>,
}

impl EventEmitter for WebsocketNotifyEventEmitter {
    fn emitter_id(&self) -> EmitterId {
        self.emitter_id
    }

    fn fire(&mut self, msg: WsMessage) {
        if let Err(e) = self.out_tx.send(msg) {
            log::error!("[WebsocketEventEmitter] send event error {e:?}");
        }
    }
}
