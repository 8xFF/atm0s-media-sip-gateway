use std::{net::SocketAddr, sync::Arc, time::Duration};

use clap::Parser;
use rust_sip_wp::{AddressBookStorage, AddressBookSync, Gateway, GatewayConfig, GatewayError, SecureContext};

/// Sip Gateway for atm0s-media-server
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// UDP/TCP port for serving QUIC/TCP connection for SDN network
    #[arg(env, long)]
    sdn_peer_id: Option<u64>,

    /// UDP/TCP port for serving QUIC/TCP connection for SDN network
    #[arg(env, long, default_value = "0.0.0.0:0")]
    sdn_listener: SocketAddr,

    /// Seeds
    #[arg(env, long, value_delimiter = ',')]
    sdn_seeds: Vec<String>,

    /// Allow it broadcast address to other peers
    /// This allows other peer can active connect to this node
    /// This option is useful with high performance relay node
    #[arg(env, long)]
    sdn_advertise_address: Option<SocketAddr>,

    /// Sdn secure code
    #[arg(env, long, default_value = "insecure")]
    sdn_secure_code: String,

    /// Listen Address for http server
    #[arg(long, env, default_value = "0.0.0.0:8008")]
    http_addr: SocketAddr,

    /// Public URL for http server
    #[arg(long, env, default_value = "http://127.0.0.1:8008")]
    http_public: String,

    /// Address for sip server
    #[arg(long, env, default_value = "0.0.0.0:5060")]
    sip_addr: SocketAddr,

    /// Secret of this gateway
    #[arg(long, env, default_value = "insecure")]
    secret: String,

    /// Address PhoneBook sync for incoming calls
    #[arg(long, env)]
    phone_numbers_sync: Option<String>,

    /// Address PhoneBook sync interval
    #[arg(long, env, default_value_t = 30_000)]
    phone_numbers_sync_interval_ms: u64,

    /// Http hook queues
    #[arg(long, env, default_value_t = 20)]
    http_hook_queues: usize,

    /// MediaServer Gateway
    #[arg(long, env)]
    media_gateway: String,

    /// MediaServer Apps sync endpoint
    #[arg(long, env)]
    media_app_sync: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), GatewayError> {
    rustls::crypto::ring::default_provider().install_default().expect("should install ring as default");
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    log::info!("Starting server with addr {}, public endpoint {} and sip port {}", args.http_addr, args.http_public, args.sip_addr);

    let secure_ctx = Arc::new(SecureContext::new(&args.secret));

    let address_book = AddressBookStorage::default();
    if let Some(sync_url) = args.phone_numbers_sync {
        let mut address_book_sync = AddressBookSync::new(&sync_url, Duration::from_millis(args.phone_numbers_sync_interval_ms), address_book.clone());

        tokio::spawn(async move {
            address_book_sync.run_loop().await;
        });
    }

    let cfg = GatewayConfig {
        http_addr: args.http_addr,
        sip_addr: args.sip_addr,
        address_book,
        http_hook_queues: args.http_hook_queues,
        media_gateway: args.media_gateway,
        secure_ctx,
        sdn_peer_id: args.sdn_peer_id.unwrap_or_else(rand::random).into(),
        sdn_listen_addr: args.sdn_listener,
        sdn_advertise: args.sdn_advertise_address.map(|a| a.into()),
        sdn_seeds: args.sdn_seeds.iter().map(|s| s.parse().expect("should convert to address")).collect::<Vec<_>>(),
        sdn_secret: args.sdn_secure_code,
    };
    let mut gateway = Gateway::new(cfg).await?;
    loop {
        if let Err(e) = gateway.recv().await {
            log::error!("gateway error {e:?}");
        }
    }
}
