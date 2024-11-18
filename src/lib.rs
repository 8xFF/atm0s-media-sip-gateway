use std::{
    io,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use atm0s_small_p2p::{pubsub_service::PubsubService, NetworkAddress, P2pNetwork, P2pNetworkConfig, P2pNetworkEvent, PeerAddress, PeerId, SharedKeyHandshake};
use call_manager::CallManager;
use clap::ValueEnum;
use hook::HttpHook;
use http::{HttpCommand, HttpServer};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use thiserror::Error;
use tokio::sync::mpsc::Receiver;
use utils::select3;

mod address_book;
mod call_manager;
mod error;
mod hook;
mod http;
mod protocol;
mod secure;
mod sip;
mod utils;

pub use address_book::{AddressBookStorage, AddressBookSync};
pub use secure::SecureContext;

pub const DEFAULT_CLUSTER_CERT: &[u8] = include_bytes!("../certs/dev.cluster.cert");
pub const DEFAULT_CLUSTER_KEY: &[u8] = include_bytes!("../certs/dev.cluster.key");

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("IoError {0}")]
    Io(#[from] io::Error),
    #[error("SipError {0}")]
    Sip(#[from] sip::SipServerError),
    #[error("QueueError")]
    Queue,
    #[error("Anyhow({0})")]
    Anyhow(#[from] anyhow::Error),
    #[error("ReqwestError {0}")]
    Reqwest(#[from] reqwest::Error),
}

pub struct GatewayConfig {
    pub http_addr: SocketAddr,
    pub public_ip: IpAddr,
    pub sip_addr: SocketAddr,
    pub address_book: AddressBookStorage,
    pub http_hook_queues: usize,
    pub media_gateway: String,
    pub secure_ctx: Arc<SecureContext>,
    pub sdn_peer_id: PeerId,
    pub sdn_listen_addr: SocketAddr,
    pub sdn_seeds: Vec<PeerAddress>,
    pub sdn_secret: String,
}

pub struct Gateway {
    http_rx: Receiver<HttpCommand>,
    call_manager: CallManager,
    p2p: P2pNetwork<SharedKeyHandshake>,
}

impl Gateway {
    pub async fn new(cfg: GatewayConfig) -> Result<Self, GatewayError> {
        let priv_key = PrivatePkcs8KeyDer::from(DEFAULT_CLUSTER_KEY.to_vec());
        let cert = CertificateDer::from(DEFAULT_CLUSTER_CERT.to_vec());

        let advertise_addr = NetworkAddress::from(SocketAddr::new(cfg.public_ip, cfg.sdn_listen_addr.port()));
        let node_addr = PeerAddress::new(cfg.sdn_peer_id, advertise_addr.clone());
        let mut p2p = P2pNetwork::new(P2pNetworkConfig {
            peer_id: cfg.sdn_peer_id,
            listen_addr: cfg.sdn_listen_addr,
            advertise: Some(advertise_addr),
            priv_key,
            cert,
            tick_ms: 1000,
            seeds: cfg.sdn_seeds,
            secure: SharedKeyHandshake::from(cfg.sdn_secret.as_str()),
        })
        .await?;

        let mut pubsub_call = PubsubService::new(p2p.create_service(0.into()));
        let p2p_pubsub_call = pubsub_call.requester();
        let http_hook = HttpHook::new(cfg.http_hook_queues);

        let (mut http, http_rx) = HttpServer::new(cfg.http_addr, node_addr.clone(), &cfg.media_gateway, cfg.secure_ctx.clone(), p2p_pubsub_call.clone());
        tokio::spawn(async move { http.run_loop().await });
        tokio::spawn(async move { while let Ok(_) = pubsub_call.run_loop().await {} });

        Ok(Self {
            http_rx,
            call_manager: CallManager::new(p2p_pubsub_call, cfg.sip_addr, cfg.public_ip, cfg.address_book, cfg.secure_ctx, http_hook, &cfg.media_gateway).await,
            p2p,
        })
    }

    pub async fn recv(&mut self) -> Result<(), GatewayError> {
        let out = select3::or(self.http_rx.recv(), self.p2p.recv(), self.call_manager.recv()).await;
        match out {
            select3::OrOutput::Left(cmd) => match cmd.expect("internal channel error") {
                HttpCommand::CreateCall(req, media_api, sender) => {
                    let res = self.call_manager.create_call(req, media_api);
                    if let Err(e) = sender.send(res) {
                        log::warn!("[Gateway] sending create_call response error {e:?}");
                    }
                    Ok(())
                }
            },
            select3::OrOutput::Middle(out) => match out? {
                P2pNetworkEvent::PeerConnected(_, peer_id) => {
                    log::info!("[Gateway] peer {peer_id} connected");
                    Ok(())
                }
                P2pNetworkEvent::PeerDisconnected(_, peer_id) => {
                    log::info!("[Gateway] peer {peer_id} disconnected");
                    Ok(())
                }
                P2pNetworkEvent::Continue => Ok(()),
            },
            select3::OrOutput::Right(_) => Ok(()),
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CloudProvider {
    Aws,
    Gcp,
    Azure,
    Other,
}

pub async fn fetch_public_ip_from_cloud(cloud: CloudProvider) -> Result<IpAddr, String> {
    match cloud {
        CloudProvider::Aws => {
            let resp = reqwest::get("http://169.254.169.254/latest/meta-data/public-ipv4").await.map_err(|e| e.to_string())?;
            let ip = resp.text().await.map_err(|e| e.to_string())?;
            IpAddr::from_str(ip.trim()).map_err(|e| e.to_string())
        }
        CloudProvider::Gcp => {
            let client = reqwest::Client::new();
            let resp = client
                .get("http://metadata/computeMetadata/v1/instance/network-interfaces/0/access-configs/0/external-ip")
                .header("Metadata-Flavor", "Google")
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let ip = resp.text().await.map_err(|e| e.to_string())?;
            IpAddr::from_str(ip.trim()).map_err(|e| e.to_string())
        }
        CloudProvider::Azure | CloudProvider::Other => {
            let resp = reqwest::get("http://ipv4.icanhazip.com").await.map_err(|e| e.to_string())?;
            let ip = resp.text().await.map_err(|e| e.to_string())?;
            IpAddr::from_str(ip.trim()).map_err(|e| e.to_string())
        }
    }
}
