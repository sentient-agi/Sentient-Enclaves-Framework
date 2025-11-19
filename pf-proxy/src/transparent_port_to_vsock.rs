// Based on:
// https://github.com/tokio-rs/tokio/blob/master/examples/proxy.rs
// https://github.com/tokio-rs/tokio/blob/tokio-1.43.0/examples/proxy.rs
// https://github.com/rust-vsock/tokio-vsock
// https://github.com/rust-vsock/vsock-rs

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use futures::FutureExt;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_vsock::{VsockAddr, VsockStream};
use tracing::{error, info};

use pf_proxy::addr_info::AddrInfo;

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
    info!(listen_addr = %listen_addr, "Starting transparent port-to-vsock proxy");
    info!(cid = %cid, "Forwarding to vsock CID");

    let listener = TcpListener::bind(listen_addr)
        .await
        .context("Failed to bind listener: malformed listening address:port")?;

    while let Ok((inbound, _)) = listener.accept().await {
        let transfer = transfer(inbound, cid).map(|r| {
            if let Err(e) = r {
                error!(error = ?e, "Connection transfer failed");
            }
        });

        tokio::spawn(transfer);
    }

    Ok(())
}

async fn transfer(mut inbound: TcpStream, cid: u32) -> Result<()> {
    let inbound_addr = inbound
        .peer_addr()
        .context("could not fetch inbound address from TCP stream")?
        .to_string();

    // Read original destination from inbound TCP stream
    let orig_dst = inbound
        .get_original_dst()
        .ok_or(anyhow!("Failed to retrieve original destination from TCP stream"))?;
    info!(orig_dst = ?orig_dst, ip = ?orig_dst.ip(), port = %orig_dst.port(), "Retrieved original destination");

    let proxy_addr = VsockAddr::new(cid, orig_dst.port().into());
    info!(proxy_addr = ?proxy_addr, "Connecting to vsock endpoint");

    let mut outbound = VsockStream::connect(proxy_addr)
        .await
        .context("failed to connect to vsock endpoint")?;

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    // Send request to upstream resource
    let client_to_server = async {
        io::copy(&mut ri, &mut wo)
            .await
            .context("error in port to vsock copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        info!(from = %inbound_addr, to = ?proxy_addr, orig_dst = ?orig_dst, ip = ?orig_dst.ip(), port = %orig_dst.port(), direction = "port->vsock", "Data transfer completed");
        wo.shutdown().await
    };

    // Receive response from upstream resource and write it to inbound connection input stream
    let server_to_client = async {
        io::copy(&mut ro, &mut wi)
            .await
            .context("error in vsock to port copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        info!(from = ?proxy_addr, to = %inbound_addr, orig_dst = ?orig_dst, ip = ?orig_dst.ip(), port = %orig_dst.port(), direction = "vsock->port", "Data transfer completed");
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client).with_context(|| {
        format!(
            "error in connection between inbound ip address {:?} with original_dst={:?}, ip={:?}, port={:?}, and outbound vsock endpoint {:?} with port={:?}",
            inbound_addr, orig_dst, orig_dst.ip(), orig_dst.port(), proxy_addr, orig_dst.port()
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
    port_to_vsock(&cli.ip_addr, cli.vsock).await?;

    Ok(())
}
