// Based on:
// https://github.com/tokio-rs/tokio/blob/master/examples/proxy.rs
// https://github.com/tokio-rs/tokio/blob/tokio-1.43.0/examples/proxy.rs
// https://github.com/rust-vsock/tokio-vsock
// https://github.com/rust-vsock/vsock-rs

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use futures::FutureExt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_vsock::{VsockAddr, VsockListener, VsockStream};
use tracing::{error, info};

use pf_proxy::utils;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// vsock address of the listener side, usually open to the other side of the transparent proxy (e.g. 3:1200)
    #[clap(short, long, value_parser)]
    vsock_addr: String,
}

pub async fn proxy(listen_addr: VsockAddr) -> Result<()> {
    info!(listen_addr = ?listen_addr, "Starting vsock-to-ip transparent proxy");

    let mut listener = VsockListener::bind(listen_addr)
        .context("Failed to bind listener to vsock: incorrect CID:port")?;

    while let Ok((inbound, _)) = listener.accept().await {
        let transfer = transfer(inbound).map(|r| {
            if let Err(e) = r {
                error!(error = ?e, "Connection transfer failed");
            }
        });

        tokio::spawn(transfer);
    }

    Ok(())
}

async fn transfer(mut inbound: VsockStream) -> Result<()> {
    let inbound_addr = inbound
        .peer_addr()
        .context("could not fetch inbound address from vsock stream")?
        .to_string();

    let (mut ri, mut wi) = inbound.split();

    // read original destination ip and port through vsock stream sent from enclave's proxy
    let proxy_addr = match ri.read_u8().await? {
        4_u8 => Ok(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::from_bits(ri.read_u32_le().await?)),
            ri.read_u16_le().await?
        )),
        6_u8 => Ok(SocketAddr::new(
            IpAddr::V6(Ipv6Addr::from_bits(ri.read_u128_le().await?)),
            ri.read_u16_le().await?
        )),
        _ => Err(anyhow!("Can't retrieve original_dst from vsock stream: malformed bytes or its order of original_dst address:port")),
    }?;

    /*
    // read original destination ip and port through vsock stream sent from enclave's proxy
    let proxy_addr = SocketAddr::new(
        IpAddr::V4(ri.read_u32_le().await?.into()),
        ri.read_u16_le().await?,
    );
    */

    info!(proxy_addr = ?proxy_addr, "Connecting to IP endpoint");

    let mut outbound = TcpStream::connect(proxy_addr)
        .await
        .context("failed to connect to TCP endpoint")?;

    let (mut ro, mut wo) = outbound.split();

    // Send request to upstream resource
    let client_to_server = async {
        io::copy(&mut ri, &mut wo)
            .await
            .context("error in vsock to ip copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        info!(from = %inbound_addr, to = ?proxy_addr, direction = "vsock->ip", "Data transfer completed");
        wo.shutdown().await
    };

    // Receive response from upstream resource and write it to inbound connection input stream
    let server_to_client = async {
        io::copy(&mut ro, &mut wi)
            .await
            .context("error in ip to vsock copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        info!(from = ?proxy_addr, to = %inbound_addr, direction = "ip->vsock", "Data transfer completed");
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
    proxy(vsock_addr).await?;

    Ok(())
}
