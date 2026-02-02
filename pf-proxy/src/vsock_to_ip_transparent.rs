// Based on:
// https://github.com/tokio-rs/tokio/blob/master/examples/proxy.rs
// https://github.com/tokio-rs/tokio/blob/tokio-1.43.0/examples/proxy.rs
// https://github.com/rust-vsock/tokio-vsock
// https://github.com/rust-vsock/vsock-rs

use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_vsock::{VsockAddr, VsockListener, VsockStream};
use tracing::{debug, error, info};

use pf_proxy::utils;

#[derive(thiserror::Error, Debug)]
pub enum ProxyError {
    #[error("failed to bind vsock listener on {addr:?}")]
    BindError {
        addr: VsockAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to accept incoming vsock connection")]
    AcceptError(#[source] std::io::Error),
    #[error("could not fetch peer address from vsock stream")]
    PeerAddrError(#[source] std::io::Error),
    #[error("failed to read IP version marker from vsock stream")]
    ReadIpVersionError(#[source] std::io::Error),
    #[error("failed to read IPv4 address from vsock stream")]
    ReadIpv4Error(#[source] std::io::Error),
    #[error("failed to read IPv6 address from vsock stream")]
    ReadIpv6Error(#[source] std::io::Error),
    #[error("failed to read port from vsock stream")]
    ReadPortError(#[source] std::io::Error),
    #[error("can't retrieve original_dst from vsock stream: malformed bytes or invalid IP version marker {marker}")]
    MalformedOriginalDst { marker: u8 },
    #[error("failed to connect to TCP endpoint {addr}")]
    TcpConnectError {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("error during data transfer between {inbound} and {outbound}")]
    TransferError {
        inbound: String,
        outbound: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("error in vsock to ip copy")]
    VsockToIpCopyError(#[source] std::io::Error),
    #[error("error in ip to vsock copy")]
    IpToVsockCopyError(#[source] std::io::Error),
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// vsock address of the listener side, usually open to the other side of the transparent proxy (e.g. 3:1200)
    #[clap(short, long, value_parser)]
    vsock_addr: String,
}

pub async fn proxy(listen_addr: VsockAddr) -> Result<()> {
    info!("Listening on: {:?}", listen_addr);

    let mut listener = VsockListener::bind(listen_addr)
        .map_err(|e| ProxyError::BindError {
            addr: listen_addr,
            source: e,
        })
        .context("Failed to bind listener to vsock: incorrect CID:port")?;

    debug!("Vsock listener bound successfully on {:?}", listen_addr);

    loop {
        match listener.accept().await {
            Ok((inbound, peer_addr)) => {
                debug!("Accepted vsock connection from {:?}", peer_addr);
                let transfer = transfer(inbound).map(|r| {
                    if let Err(e) = r {
                        error!("Failed to transfer data: error={:?}", e);
                    }
                });

                tokio::spawn(transfer);
            }
            Err(e) => {
                error!("Failed to accept vsock connection: {:?}", e);
                continue;
            }
        }
    }
}

async fn transfer(mut inbound: VsockStream) -> Result<()> {
    let inbound_addr = inbound
        .peer_addr()
        .map_err(ProxyError::PeerAddrError)
        .context("could not fetch inbound address from vsock stream")?
        .to_string();

    debug!("Processing vsock connection from {}", inbound_addr);

    let (mut ri, mut wi) = inbound.split();

    // read original destination ip and port through vsock stream sent from enclave's proxy
    debug!("Reading original destination from vsock stream");

    let ip_version = ri
        .read_u8()
        .await
        .map_err(ProxyError::ReadIpVersionError)
        .context("Failed to read IP version marker")?;

    debug!("IP version marker: {}", ip_version);

    let proxy_addr = match ip_version {
        4_u8 => {
            let ip_bits = ri
                .read_u32_le()
                .await
                .map_err(ProxyError::ReadIpv4Error)
                .context("Failed to read IPv4 address")?;
            let port = ri
                .read_u16_le()
                .await
                .map_err(ProxyError::ReadPortError)
                .context("Failed to read port")?;
            debug!("Read IPv4 address: {:?}, port: {}", Ipv4Addr::from_bits(ip_bits), port);
            Ok(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::from_bits(ip_bits)),
                port,
            ))
        }
        6_u8 => {
            let ip_bits = ri
                .read_u128_le()
                .await
                .map_err(ProxyError::ReadIpv6Error)
                .context("Failed to read IPv6 address")?;
            let port = ri
                .read_u16_le()
                .await
                .map_err(ProxyError::ReadPortError)
                .context("Failed to read port")?;
            debug!("Read IPv6 address: {:?}, port: {}", Ipv6Addr::from_bits(ip_bits), port);
            Ok(SocketAddr::new(
                IpAddr::V6(Ipv6Addr::from_bits(ip_bits)),
                port,
            ))
        }
        marker => {
            error!("Invalid IP version marker received: {}", marker);
            Err(ProxyError::MalformedOriginalDst { marker })
                .context("Can't retrieve original_dst from vsock stream: malformed bytes or its order of original_dst address:port")
        }
    }?;

    info!("Proxying to: {:?}", proxy_addr);

    let mut outbound = TcpStream::connect(proxy_addr)
        .await
        .map_err(|e| ProxyError::TcpConnectError {
            addr: proxy_addr,
            source: e,
        })
        .context("failed to connect to TCP endpoint")?;

    debug!("Connected to TCP endpoint {:?}", proxy_addr);

    let (mut ro, mut wo) = outbound.split();

    // Send request to upstream resource
    let client_to_server = async {
        let result = io::copy(&mut ri, &mut wo).await;
        match &result {
            Ok(bytes) => debug!("Copied {} bytes from vsock to ip", bytes),
            Err(e) => error!("Error copying from vsock to ip: {:?}", e),
        }
        result
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, ProxyError::VsockToIpCopyError(e)))?;
        info!(
            "vsock to ip IO copy done, from {:?} to {:?}",
            inbound_addr, proxy_addr
        );
        wo.shutdown().await
    };

    // Receive response from upstream resource and write it to inbound connection input stream
    let server_to_client = async {
        let result = io::copy(&mut ro, &mut wi).await;
        match &result {
            Ok(bytes) => debug!("Copied {} bytes from ip to vsock", bytes),
            Err(e) => error!("Error copying from ip to vsock: {:?}", e),
        }
        result
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, ProxyError::IpToVsockCopyError(e)))?;
        info!(
            "ip to vsock IO copy done, from {:?} to {:?}",
            proxy_addr, inbound_addr
        );
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client).with_context(|| {
        format!(
            "error in connection between inbound vsock endpoint {:?} and outbound ip address {:?}",
            inbound_addr, proxy_addr
        )
    })?;

    debug!(
        "Transfer completed successfully between {} and {:?}",
        inbound_addr, proxy_addr
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

    info!("Starting vsock_to_ip_transparent proxy");
    proxy(vsock_addr).await?;

    Ok(())
}
