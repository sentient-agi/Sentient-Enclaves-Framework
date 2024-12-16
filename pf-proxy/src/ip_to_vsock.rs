// Based on https://github.com/tokio-rs/tokio/blob/master/examples/proxy.rs
//
// Copyright (c) 2022 Tokio Contributors and Marlin Contributors
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_vsock::{VsockAddr, VsockStream};

use pf_proxy::utils;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// ip address of the listener side (e.g. 0.0.0.0:4000)
    #[clap(short, long, value_parser)]
    ip_addr: String,

    /// vsock address of the upstream side (e.g. 88:4000)
    #[clap(short, long, value_parser)]
    vsock_addr: String,
}

pub async fn proxy(listen_addr: &str, server_addr: VsockAddr) -> Result<()> {
    println!("Listening on: {:?}", listen_addr);
    println!("Proxying to: {:?}", server_addr);

    let listener = TcpListener::bind(listen_addr)
        .await
        .context("Failed to bind listener: malformed listening address:port")?;

    while let Ok((inbound, _)) = listener.accept().await {
        let transfer = transfer(inbound, server_addr).map(|r| {
            if let Err(e) = r {
                println!("Failed to transfer data: error={:?}", e);
            }
        });

        tokio::spawn(transfer);
    }

    Ok(())
}

async fn transfer(mut inbound: TcpStream, proxy_addr: VsockAddr) -> Result<()> {
    let inbound_addr = inbound
        .peer_addr()
        .context("could not fetch inbound address from TCP stream")?
        .to_string();

    println!("Proxying to: {:?}", proxy_addr);

    let mut outbound = VsockStream::connect(proxy_addr)
        .await
        .context("failed to connect to vsock endpoint")?;

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    // Send request to upstream resource
    let client_to_server = async {
        io::copy(&mut ri, &mut wo)
            .await
            .context("error in ip to vsock copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        println!("ip to vsock IO copy done, from {:?} to {:?}", inbound_addr, proxy_addr);
        wo.shutdown().await
    };

    // Receive response from upstream resource and write it to inbound connection input stream
    let server_to_client = async {
        io::copy(&mut ro, &mut wi)
            .await
            .context("error in vsock to ip copy")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        println!("vsock to ip IO copy done, from {:?} to {:?}", proxy_addr, inbound_addr);
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client).with_context(|| {
        format!(
            "error in connection between inbound ip address {:?} and outbound vsock endpoint {:?}",
            inbound_addr, proxy_addr
        )
    })?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let vsock_addr = utils::split_vsock(&cli.vsock_addr)?;
    proxy(&cli.ip_addr, vsock_addr).await?;

    Ok(())
}
