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
use tracing::{error, info};

use pf_proxy::utils;

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
    info!(listen_addr = ?listen_addr, "Starting vsock-to-ip proxy");
    info!(server_addr = %server_addr, "Forwarding to IP address");

    let mut listener = VsockListener::bind(listen_addr)
        .context("Failed to bind listener to vsock: incorrect CID:port")?;

    while let Ok((inbound, _)) = listener.accept().await {
        let transfer = transfer(inbound, server_addr.clone()).map(|r| {
            if let Err(e) = r {
                error!(error = ?e, "Connection transfer failed");
            }
        });

        tokio::spawn(transfer);
    }

    Ok(())
}

async fn transfer(mut inbound: VsockStream, proxy_addr: String) -> Result<()> {
    let inbound_addr = inbound
        .local_addr() // .peer_addr()
        .context("could not fetch inbound address from vsock stream")?
        .to_string();

    info!(from = %inbound_addr, to = %proxy_addr, "New connection established");

    let mut outbound = TcpStream::connect(proxy_addr.clone())
        .await
        .context("failed to connect to TCP endpoint")?;

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    // Send request to upstream resource
    let client_to_server = async {
        io::copy(&mut ri, &mut wo)
            .await
            .context("error in vsock to ip copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        info!(from = %inbound_addr, to = %proxy_addr, direction = "vsock->ip", "Data transfer completed");
        wo.shutdown().await
    };

    // Receive response from upstream resource and write it to inbound connection input stream
    let server_to_client = async {
        io::copy(&mut ro, &mut wi)
            .await
            .context("error in ip to vsock copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        info!(from = %proxy_addr, to = %inbound_addr, direction = "ip->vsock", "Data transfer completed");
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client).with_context(|| {
        format!(
            "error in connection between inbound vsock endpoint {:?} and outbound ip address {:?}",
            inbound_addr, proxy_addr
        )
    })?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let cli = Cli::parse();
    let vsock_addr = utils::split_vsock(&cli.vsock_addr)?;
    proxy(vsock_addr, cli.ip_addr).await?;

    Ok(())
}
