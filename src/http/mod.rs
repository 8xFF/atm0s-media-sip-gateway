use std::{io, net::SocketAddr, sync::Arc};

use crate::{
    protocol::{CallApiError, CreateCallRequest, CreateCallResponse},
    secure::SecureContext,
    sip::MediaApi,
};
use atm0s_small_p2p::pubsub_service::PubsubServiceRequester;
use poem::{get, listener::TcpListener, middleware::Tracing, EndpointExt, Route, Server};
use poem_openapi::OpenApiService;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

mod api_call;
mod header_secret;
mod response_result;
mod ws_in_call;
mod ws_out_call;

pub enum HttpCommand {
    CreateCall(CreateCallRequest, MediaApi, oneshot::Sender<Result<CreateCallResponse, CallApiError>>),
}

pub struct HttpServer {
    addr: SocketAddr,
    media_gateway: String,
    secure_ctx: Arc<SecureContext>,
    tx: Sender<HttpCommand>,
    call_pubsub: PubsubServiceRequester,
}

impl HttpServer {
    pub fn new(addr: SocketAddr, media_gateway: &str, secure_ctx: Arc<SecureContext>, call_pubsub: PubsubServiceRequester) -> (Self, Receiver<HttpCommand>) {
        let (tx, rx) = channel(10);
        (
            Self {
                addr,
                media_gateway: media_gateway.to_owned(),
                tx,
                secure_ctx,
                call_pubsub,
            },
            rx,
        )
    }

    pub async fn run_loop(&mut self) -> io::Result<()> {
        let call_api = api_call::CallApis {
            media_gateway: self.media_gateway.clone(),
            tx: self.tx.clone(),
            secure_ctx: self.secure_ctx.clone(),
            call_pubsub: self.call_pubsub.clone(),
        };
        let call_service: OpenApiService<_, ()> = OpenApiService::new(call_api, "Console call APIs", env!("CARGO_PKG_VERSION")).server("/").url_prefix("/call");
        let call_ui = call_service.swagger_ui();
        let call_spec = call_service.spec();

        let app = Route::new()
            .nest("/call/", call_service)
            .nest("/docs/call/", call_ui)
            .at("/docs/call/spec", poem::endpoint::make_sync(move |_| call_spec.clone()))
            .at(
                "/call/outgoing/:call_id",
                get(ws_out_call::ws_single_call).data(ws_out_call::WebsocketCallCtx {
                    secure_ctx: self.secure_ctx.clone(),
                    call_pubsub: self.call_pubsub.clone(),
                }),
            )
            .at(
                "/call/incoming/:call_id",
                get(ws_in_call::ws_single_call).data(ws_in_call::WebsocketCallCtx {
                    secure_ctx: self.secure_ctx.clone(),
                    call_pubsub: self.call_pubsub.clone(),
                }),
            )
            .with(Tracing::default());

        Server::new(TcpListener::bind(self.addr)).run(app).await
    }
}
