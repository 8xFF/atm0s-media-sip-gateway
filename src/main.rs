use std::{
    net::{IpAddr, SocketAddr, UdpSocket},
    sync::Arc,
    time::Duration,
};

use atm0s_media_sip_gateway::{fetch_public_ip_from_cloud, AddressBookStorage, AddressBookSync, CloudProvider, Gateway, GatewayConfig, GatewayError, SecureContext};
use clap::Parser;

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

    /// Seed from other node-api
    #[arg(env, long)]
    sdn_seeds_from_url: Option<String>,

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

    /// Allow it broadcast address to other peers or sip-servers
    /// This allows other peer can active connect to this node
    #[arg(long, env, default_value = "127.0.0.1")]
    public_ip: IpAddr,

    /// Gather public ip from cloud provider
    #[arg(long, env)]
    public_ip_cloud: Option<CloudProvider>,

    /// Secret of this gateway
    #[arg(long, env, default_value = "insecure")]
    secret: String,

    /// Address PhoneBook sync for incoming calls
    #[arg(long, env)]
    phone_numbers_sync: Option<String>,

    /// Address PhoneBook sync for incoming calls
    #[arg(long, env)]
    apps_sync: Option<String>,

    /// Address PhoneBook sync interval
    #[arg(long, env, default_value_t = 30_000)]
    sync_interval_ms: u64,

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
    let mut args = Args::parse();

    if args.sdn_listener.port() == 0 {
        let udp = UdpSocket::bind(args.sdn_listener)?;
        args.sdn_listener.set_port(udp.local_addr()?.port());
    }

    log::info!("Starting server with addr {}, public endpoint {} and sip port {}", args.http_addr, args.http_public, args.sip_addr);

    let address_book = AddressBookStorage::new(&args.secret);
    let secure_ctx = Arc::new(SecureContext::new(&args.secret, address_book.clone()));

    if let Some(phone_url) = args.phone_numbers_sync {
        if let Some(app_url) = args.apps_sync {
            let mut address_book_sync = AddressBookSync::new(&phone_url, &app_url, Duration::from_millis(args.sync_interval_ms), address_book.clone());

            tokio::spawn(async move {
                address_book_sync.run_loop().await;
            });
        }
    }

    let mut other_node_addr = vec![];
    if let Some(sdn_seeds_from_url) = args.sdn_seeds_from_url {
        log::info!("Fetching seeds from node api {sdn_seeds_from_url}");
        let addr = reqwest::get(&sdn_seeds_from_url).await?.text().await?;
        other_node_addr.push(addr);
        log::info!("Fetched seeds: {other_node_addr:?}");
    }

    let mut public_ip_cloud = None;
    if let Some(cloud) = args.public_ip_cloud {
        log::info!("Fetching public ip from cloud provider {cloud:?}");
        public_ip_cloud = Some(fetch_public_ip_from_cloud(cloud).await.expect("should fetch public ip from cloud"));
        log::info!("Fetched public ip: {public_ip_cloud:?}");
    }

    let cfg = GatewayConfig {
        http_addr: args.http_addr,
        public_ip: public_ip_cloud.unwrap_or(args.public_ip),
        sip_addr: args.sip_addr,
        address_book,
        http_hook_queues: args.http_hook_queues,
        media_gateway: args.media_gateway,
        secure_ctx,
        sdn_peer_id: args.sdn_peer_id.unwrap_or_else(rand::random).into(),
        sdn_listen_addr: args.sdn_listener,
        sdn_seeds: other_node_addr
            .iter()
            .chain(args.sdn_seeds.iter())
            .map(|s| s.parse().expect("should convert to address"))
            .collect::<Vec<_>>(),
        sdn_secret: args.sdn_secure_code,
    };
    let mut gateway = Gateway::new(cfg).await?;
    loop {
        if let Err(e) = gateway.recv().await {
            log::error!("gateway error {e:?}");
        }
    }
}
