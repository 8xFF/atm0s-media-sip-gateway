use std::{io, net::SocketAddr, sync::Arc};

use address_book::AddressBookStorage;
use call_manager::CallManager;
use futures::select2;
use hook::HttpHook;
use http::{HttpCommand, HttpServer, WebsocketEventEmitter};
use secure::SecureContext;
use thiserror::Error;
use tokio::sync::mpsc::Receiver;

pub mod address_book;
pub mod call_manager;
mod error;
pub mod futures;
pub mod hook;
pub mod http;
pub mod protocol;
pub mod secure;
pub mod sip;
pub mod utils;

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("IoError {0}")]
    Io(#[from] io::Error),
    #[error("SipError {0}")]
    Sip(#[from] sip::SipServerError),
    #[error("QueueError")]
    Queue,
}

pub struct Gateway {
    http_rx: Receiver<HttpCommand>,
    call_manager: CallManager<WebsocketEventEmitter>,
}

impl Gateway {
    pub async fn new(
        http_addr: SocketAddr,
        http_public: &str,
        sip_addr: SocketAddr,
        address_book: AddressBookStorage,
        http_hook_queues: usize,
        media_gateway: &str,
        secure_ctx: Arc<SecureContext>,
    ) -> Result<Self, GatewayError> {
        let (mut http, http_rx) = HttpServer::new(http_addr, media_gateway, secure_ctx.clone());
        tokio::spawn(async move { http.run_loop().await });
        let http_hook = HttpHook::new(http_hook_queues);

        Ok(Self {
            http_rx,
            call_manager: CallManager::new(sip_addr, http_public, address_book, secure_ctx, http_hook, media_gateway).await,
        })
    }

    pub async fn recv(&mut self) -> Result<(), GatewayError> {
        let out = select2::or(self.http_rx.recv(), self.call_manager.recv()).await;
        match out {
            select2::OrOutput::Left(cmd) => match cmd.expect("internal channel error") {
                HttpCommand::CreateCall(req, media_api, sender) => {
                    let res = self.call_manager.create_call(req, media_api);
                    if let Err(e) = sender.send(res) {
                        log::warn!("[Gateway] sending create_call response error {e:?}");
                    }
                    Ok(())
                }
                HttpCommand::ActionCall(call_id, req, sender) => {
                    self.call_manager.action_call(call_id, req, sender);
                    Ok(())
                }
                HttpCommand::EndCall(call_id, sender) => {
                    let res = self.call_manager.end_call(call_id);
                    if let Err(e) = sender.send(res) {
                        log::warn!("[Gateway] sending end_call response error {e:?}");
                    }
                    Ok(())
                }
                HttpCommand::SubscribeCall(call_id, emitter, sender) => {
                    let res = self.call_manager.subscribe_call(call_id, emitter);
                    if let Err(e) = sender.send(res) {
                        log::warn!("[Gateway] sending sub_call response error {e:?}");
                    }
                    Ok(())
                }
                HttpCommand::UnsubscribeCall(call_id, emitter_id, sender) => {
                    let res = self.call_manager.unsubscribe_call(call_id, emitter_id);
                    if let Err(e) = sender.send(res) {
                        log::warn!("[Gateway] sending unsub_call response error {e:?}");
                    }
                    Ok(())
                }
            },
            select2::OrOutput::Right(_) => Ok(()),
        }
    }
}
