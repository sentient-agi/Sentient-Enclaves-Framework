// Based on:
// https://github.com/tokio-rs/tokio/blob/master/examples/proxy.rs
// https://github.com/tokio-rs/tokio/blob/tokio-1.43.0/examples/proxy.rs
// https://github.com/rust-vsock/tokio-vsock
// https://github.com/rust-vsock/vsock-rs

use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use tokio::io;
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
    #[error("could not fetch local address from vsock stream")]
    LocalAddrError(#[source] std::io::Error),
    #[error("failed to connect to TCP endpoint {addr}")]
    TcpConnectError {
        addr: String,
        #[source]
        source: std::io::Error,
    },
    #[error("error during data transfer between {inbound} and {outbound}")]
    TransferError {
        inbound: String,
        outbound: String,
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
    /// vsock address of the listener side (e.g. 88:4000)
    #[clap(short, long, value_parser)]
    vsock_addr: String,

    /// ip address of the upstream side (e.g. 127.0.0.1:4000)
    #[clap(short, long, value_parser)]
    ip_addr: String,
}

pub async fn proxy(listen_addr: VsockAddr, server_addr: String) -> Result<()> {
    info!("Listening on: {:?}", listen_addr);
    info!("Proxying to: {:?}", server_addr);

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
                let server_addr_clone = server_addr.clone();
                let transfer = transfer(inbound, server_addr_clone).map(|r| {
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

async fn transfer(mut inbound: VsockStream, proxy_addr: String) -> Result<()> {
    let inbound_addr = inbound
        .local_addr()
        .map_err(ProxyError::LocalAddrError)
        .context("could not fetch inbound address from vsock stream")?
        .to_string();

    debug!("Processing vsock connection from {}", inbound_addr);
    info!("Proxying to: {:?}", proxy_addr);

    let mut outbound = TcpStream::connect(proxy_addr.clone())
        .await
        .map_err(|e| ProxyError::TcpConnectError {
            addr: proxy_addr.clone(),
            source: e,
        })
        .context("failed to connect to TCP endpoint")?;

    debug!("Connected to TCP endpoint {}", proxy_addr);

    let (mut ri, mut wi) = inbound.split();
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
        "Transfer completed successfully between {} and {}",
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

    info!("Starting vsock_to_ip proxy");
    proxy(vsock_addr, cli.ip_addr).await?;

    Ok(())
}
