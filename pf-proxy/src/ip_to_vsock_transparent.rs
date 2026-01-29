// Based on:
// https://github.com/tokio-rs/tokio/blob/master/examples/proxy.rs
// https://github.com/tokio-rs/tokio/blob/tokio-1.43.0/examples/proxy.rs
// https://github.com/rust-vsock/tokio-vsock
// https://github.com/rust-vsock/vsock-rs

use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use std::net::SocketAddr;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_vsock::{VsockAddr, VsockStream};
use tracing::{debug, error, info};

use pf_proxy::{addr_info::AddrInfo, utils};

#[derive(thiserror::Error, Debug)]
pub enum ProxyError {
    #[error("failed to bind TCP listener on {addr}")]
    BindError {
        addr: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to accept incoming TCP connection")]
    AcceptError(#[source] std::io::Error),
    #[error("could not fetch peer address from TCP stream")]
    PeerAddrError(#[source] std::io::Error),
    #[error("failed to retrieve original destination from TCP stream")]
    OriginalDstError,
    #[error("failed to connect to vsock endpoint {addr:?}")]
    VsockConnectError {
        addr: VsockAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write IP version marker")]
    WriteIpVersionError(#[source] std::io::Error),
    #[error("failed to write IPv4 address bits")]
    WriteIpv4Error(#[source] std::io::Error),
    #[error("failed to write IPv6 address bits")]
    WriteIpv6Error(#[source] std::io::Error),
    #[error("failed to write port")]
    WritePortError(#[source] std::io::Error),
    #[error("error during data transfer between {inbound} (orig_dst={orig_dst:?}) and {outbound:?}")]
    TransferError {
        inbound: String,
        orig_dst: SocketAddr,
        outbound: VsockAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("error in ip to vsock copy")]
    IpToVsockCopyError(#[source] std::io::Error),
    #[error("error in vsock to ip copy")]
    VsockToIpCopyError(#[source] std::io::Error),
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// ip address of the listener side (e.g. 127.0.0.1:1200)
    #[clap(short, long, value_parser)]
    ip_addr: String,

    /// vsock address of the upstream side, usually the other side of the transparent proxy (e.g. 3:1200)
    #[clap(short, long, value_parser)]
    vsock_addr: String,
}

pub async fn proxy(listen_addr: &str, server_addr: VsockAddr) -> Result<()> {
    info!("Listening on: {:?}", listen_addr);
    info!("Proxying to: {:?}", server_addr);

    let listener = TcpListener::bind(listen_addr)
        .await
        .map_err(|e| ProxyError::BindError {
            addr: listen_addr.to_string(),
            source: e,
        })
        .context("Failed to bind listener: malformed listening address:port")?;

    debug!("TCP listener bound successfully on {}", listen_addr);

    loop {
        match listener.accept().await {
            Ok((inbound, peer_addr)) => {
                debug!("Accepted connection from {:?}", peer_addr);
                let transfer = transfer(inbound, server_addr).map(|r| {
                    if let Err(e) = r {
                        error!("Failed to transfer data: error={:?}", e);
                    }
                });

                tokio::spawn(transfer);
            }
            Err(e) => {
                error!("Failed to accept connection: {:?}", e);
                continue;
            }
        }
    }
}

async fn transfer(mut inbound: TcpStream, proxy_addr: VsockAddr) -> Result<()> {
    let inbound_addr = inbound
        .peer_addr()
        .map_err(ProxyError::PeerAddrError)
        .context("could not fetch inbound address from TCP stream")?
        .to_string();

    debug!("Processing connection from {}", inbound_addr);

    // Read original destination from inbound TCP stream
    let orig_dst = inbound
        .get_original_dst()
        .ok_or(ProxyError::OriginalDstError)
        .context("Failed to retrieve original destination from TCP stream")?;

    info!("Original destination: {:?}", orig_dst);
    info!("Proxying to: {:?}", proxy_addr);

    let mut outbound = VsockStream::connect(proxy_addr)
        .await
        .map_err(|e| ProxyError::VsockConnectError {
            addr: proxy_addr,
            source: e,
        })
        .context("failed to connect to vsock endpoint")?;

    debug!("Connected to vsock endpoint {:?}", proxy_addr);

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    // send original destination ip and port to upstream proxy on host
    debug!(
        "Sending original destination info: {:?} (port={})",
        orig_dst,
        orig_dst.port()
    );

    match orig_dst {
        std::net::SocketAddr::V4(v4) => {
            debug!("Sending IPv4 marker and address: {:?}", v4.ip());
            wo.write_u8(4_u8)
                .await
                .map_err(ProxyError::WriteIpVersionError)
                .context("Failed to write IP version marker")?;
            wo.write_u32_le(v4.ip().to_bits())
                .await
                .map_err(ProxyError::WriteIpv4Error)
                .context("Failed to write IPv4 address")?;
        }
        std::net::SocketAddr::V6(v6) => {
            debug!("Sending IPv6 marker and address: {:?}", v6.ip());
            wo.write_u8(6_u8)
                .await
                .map_err(ProxyError::WriteIpVersionError)
                .context("Failed to write IP version marker")?;
            wo.write_u128_le(v6.ip().to_bits())
                .await
                .map_err(ProxyError::WriteIpv6Error)
                .context("Failed to write IPv6 address")?;
        }
    };
    wo.write_u16_le(orig_dst.port())
        .await
        .map_err(ProxyError::WritePortError)
        .context("Failed to write port")?;

    debug!("Original destination info sent successfully");

    // Send request to upstream resource
    let client_to_server = async {
        let result = io::copy(&mut ri, &mut wo).await;
        match &result {
            Ok(bytes) => debug!("Copied {} bytes from ip to vsock", bytes),
            Err(e) => error!("Error copying from ip to vsock: {:?}", e),
        }
        result
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, ProxyError::IpToVsockCopyError(e)))?;
        info!(
            "ip to vsock IO copy done, from {:?} to {:?}, with original_dst={:?} from inbound TCP stream",
            inbound_addr, proxy_addr, orig_dst
        );
        wo.shutdown().await
    };

    // Receive response from upstream resource and write it to inbound connection input stream
    let server_to_client = async {
        let result = io::copy(&mut ro, &mut wi).await;
        match &result {
            Ok(bytes) => debug!("Copied {} bytes from vsock to ip", bytes),
            Err(e) => error!("Error copying from vsock to ip: {:?}", e),
        }
        result
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, ProxyError::VsockToIpCopyError(e)))?;
        info!(
            "vsock to ip IO copy done, from {:?} to {:?}, with original_dst={:?} from inbound TCP stream",
            proxy_addr, inbound_addr, orig_dst
        );
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client).with_context(|| {
        format!(
            "error in connection between inbound ip address {:?} with original_dst={:?} and outbound vsock endpoint {:?}",
            inbound_addr, orig_dst, proxy_addr
        )
    })?;

    debug!(
        "Transfer completed successfully between {} (orig_dst={:?}) and {:?}",
        inbound_addr, orig_dst, proxy_addr
    );
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();

    debug!("Parsing vsock address: {}", cli.vsock_addr);
    let vsock_addr = utils::split_vsock(&cli.vsock_addr)
        .context("Failed to parse vsock address")?;

    info!("Starting ip_to_vsock_transparent proxy");
    proxy(&cli.ip_addr, vsock_addr).await?;

    Ok(())
}
