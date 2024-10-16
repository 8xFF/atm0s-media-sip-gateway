use std::sync::Arc;

use crate::secure::SecureContext;

use atm0s_small_p2p::pubsub_service::PubsubServiceRequester;
use futures_util::StreamExt;
use poem::{
    handler,
    web::{websocket::WebSocket, Data, Query},
    IntoResponse, Response,
};
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Clone)]
pub struct WebsocketNotifyCtx {
    pub secure_ctx: Arc<SecureContext>,
    pub notify_pubsub: PubsubServiceRequester,
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    token: String,
}

#[handler]
pub async fn ws_single_notify(Query(query): Query<WsQuery>, ws: WebSocket, data: Data<&WebsocketNotifyCtx>) -> impl IntoResponse {
    let token = query.token;
    let token = if let Some(token) = data.secure_ctx.decode_call_token(&token) {
        token
    } else {
        return Response::builder().status(StatusCode::UNAUTHORIZED).finish();
    };

    let channel = token.call_id.to_pubsub_channel();
    let mut subscriber = data.notify_pubsub.subscriber(channel).await;
    ws.on_upgrade(move |socket| async move {
        let (mut _sink, mut _stream) = socket.split();
    })
    .into_response()
}
