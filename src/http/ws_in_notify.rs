use std::sync::Arc;

use crate::{protocol::protobuf::sip_gateway::IncomingCallNotify, secure::SecureContext, utils::select2};

use atm0s_small_p2p::pubsub_service::{PubsubServiceRequester, SubscriberEventOb};
use futures_util::{SinkExt, StreamExt};
use poem::{
    handler,
    web::{
        websocket::{Message as WebsocketMessage, WebSocket},
        Data, Query,
    },
    IntoResponse, Response,
};
use prost::Message;
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
    let identify = if let Some(identify) = data.secure_ctx.decode_notify_token(&token) {
        identify
    } else {
        return Response::builder().status(StatusCode::UNAUTHORIZED).finish();
    };

    let channel = identify.to_pubsub_channel();
    let app = identify.app;
    let client = identify.client;
    let mut subscriber = data.notify_pubsub.subscriber(channel).await;
    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();
        loop {
            let out = select2::or(subscriber.recv_ob::<IncomingCallNotify>(), stream.next()).await;
            match out {
                select2::OrOutput::Left(Ok(event)) => match event {
                    SubscriberEventOb::PeerJoined(_) => {}
                    SubscriberEventOb::PeerLeaved(_) => {}
                    SubscriberEventOb::GuestPublish(data) => {
                        log::info!("[WsNotify {app}/{client}] got publisher message {data:?}");
                        let data = data.encode_to_vec();
                        log::info!("[WsNotify {app}/{client}] emit data {data:?}");
                        if let Err(e) = sink.send(WebsocketMessage::Binary(data)).await {
                            log::error!("[WsCall {app}/{client}] send data error {e:?}");
                            break;
                        }
                    }
                    _ => log::warn!("[WsNotify {app}/{client}] unsupported data {event:?}"),
                },
                select2::OrOutput::Left(Err(_e)) => {
                    break;
                }
                select2::OrOutput::Right(Some(_)) => {}
                select2::OrOutput::Right(None) => {
                    break;
                }
            }
        }
    })
    .into_response()
}
