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

use pf_proxy::addr_info::AddrInfo;

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
    #[error("error during data transfer between {inbound} (orig_dst={orig_dst:?}) and {outbound:?}")]
    TransferError {
        inbound: String,
        orig_dst: SocketAddr,
        outbound: VsockAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("error in port to vsock copy")]
    PortToVsockCopyError(#[source] std::io::Error),
    #[error("error in vsock to port copy")]
    VsockToPortCopyError(#[source] std::io::Error),
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// ip address of the listener side (e.g. 127.0.0.1:1200)
    #[clap(short, long, value_parser)]
    ip_addr: String,

    /// CID from vsock address of the upstream side (e.g. 88 from 88:1200/CID:PORT specification)
    #[clap(short, long, value_parser)]
    vsock: u32,
}

pub async fn port_to_vsock(listen_addr: &str, cid: u32) -> Result<()> {
    info!("Listening on: {:?}", listen_addr);
    info!("Proxying to CID: {:?}", cid);

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
                let transfer = transfer(inbound, cid).map(|r| {
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

async fn transfer(mut inbound: TcpStream, cid: u32) -> Result<()> {
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

    let proxy_addr = VsockAddr::new(cid, orig_dst.port().into());
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

    // Send request to upstream resource
    let client_to_server = async {
        let result = io::copy(&mut ri, &mut wo).await;
        match &result {
            Ok(bytes) => debug!("Copied {} bytes from port to vsock", bytes),
            Err(e) => error!("Error copying from port to vsock: {:?}", e),
        }
        result
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, ProxyError::PortToVsockCopyError(e)))?;
        info!(
            "port to vsock IO copy done, from {:?} to {:?}, with original_dst={:?}, ip={:?}, port={:?}, from inbound TCP stream",
            inbound_addr, proxy_addr, orig_dst, orig_dst.ip(), orig_dst.port()
        );
        wo.shutdown().await
    };

    // Receive response from upstream resource and write it to inbound connection input stream
    let server_to_client = async {
        let result = io::copy(&mut ro, &mut wi).await;
        match &result {
            Ok(bytes) => debug!("Copied {} bytes from vsock to port", bytes),
            Err(e) => error!("Error copying from vsock to port: {:?}", e),
        }
        result
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, ProxyError::VsockToPortCopyError(e)))?;
        info!(
            "vsock to port IO copy done, from {:?} to {:?}, with original_dst={:?}, ip={:?}, port={:?}, from inbound TCP stream",
            proxy_addr, inbound_addr, orig_dst, orig_dst.ip(), orig_dst.port()
        );
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client).with_context(|| {
        format!(
            "error in connection between inbound ip address {:?} with original_dst={:?}, ip={:?}, port={:?}, and outbound vsock endpoint {:?} with port={:?}",
            inbound_addr, orig_dst, orig_dst.ip(), orig_dst.port(), proxy_addr, orig_dst.port()
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

    info!("Starting transparent_port_to_vsock proxy");
    port_to_vsock(&cli.ip_addr, cli.vsock).await?;

    Ok(())
}
