use std::{io, net::SocketAddr, sync::Arc};

use crate::{
    call_manager::EmitterId,
    protocol::{CallActionRequest, CallActionResponse, CallApiError, CreateCallRequest, CreateCallResponse, InternalCallId},
    secure::SecureContext,
    sip::MediaApi,
};
use poem::{get, listener::TcpListener, middleware::Tracing, EndpointExt, Route, Server};
use poem_openapi::OpenApiService;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

mod api_call;
mod header_secret;
mod response_result;
mod ws_call;

use ws_call::WebsocketCtx;
pub use ws_call::WebsocketEventEmitter;

pub enum HttpCommand {
    CreateCall(CreateCallRequest, MediaApi, oneshot::Sender<Result<CreateCallResponse, CallApiError>>),
    ActionCall(InternalCallId, CallActionRequest, oneshot::Sender<anyhow::Result<CallActionResponse>>),
    EndCall(InternalCallId, oneshot::Sender<Result<(), CallApiError>>),
    SubscribeCall(InternalCallId, WebsocketEventEmitter, oneshot::Sender<Result<(), CallApiError>>),
    UnsubscribeCall(InternalCallId, EmitterId, oneshot::Sender<Result<(), CallApiError>>),
}

pub struct HttpServer {
    addr: SocketAddr,
    media_gateway: String,
    secure_ctx: Arc<SecureContext>,
    tx: Sender<HttpCommand>,
}

impl HttpServer {
    pub fn new(addr: SocketAddr, media_gateway: &str, secure_ctx: Arc<SecureContext>) -> (Self, Receiver<HttpCommand>) {
        let (tx, rx) = channel(10);
        (
            Self {
                addr,
                media_gateway: media_gateway.to_owned(),
                tx,
                secure_ctx,
            },
            rx,
        )
    }

    pub async fn run_loop(&mut self) -> io::Result<()> {
        let call_service: OpenApiService<_, ()> = OpenApiService::new(
            api_call::CallApis {
                media_gateway: self.media_gateway.clone(),
                tx: self.tx.clone(),
                secure_ctx: self.secure_ctx.clone(),
            },
            "Console call APIs",
            env!("CARGO_PKG_VERSION"),
        )
        .server("/")
        .url_prefix("/call");
        let call_ui = call_service.swagger_ui();
        let call_spec = call_service.spec();

        let app = Route::new()
            .nest("/call/", call_service)
            .nest("/docs/call/", call_ui)
            .at("/docs/call/spec", poem::endpoint::make_sync(move |_| call_spec.clone()))
            .at(
                "/ws/call/:call_id",
                get(ws_call::ws_single_call).data(WebsocketCtx {
                    cmd_tx: self.tx.clone(),
                    secure_ctx: self.secure_ctx.clone(),
                }),
            )
            .with(Tracing::default());

        Server::new(TcpListener::bind(self.addr)).run(app).await
    }
}
