pub mod addr_info;

pub mod utils {
    use std::num::ParseIntError;

    use tokio_vsock::VsockAddr;
    use tracing::debug;

    #[derive(thiserror::Error, Debug)]
    pub enum VsockAddrParseError {
        #[error("invalid vsock address, should contain one colon [:] sign")]
        SplitError,
        #[error("failed to parse cid as a u32")]
        CidParseError(#[source] ParseIntError),
        #[error("failed to parse port as a u32")]
        PortParseError(#[source] ParseIntError),
    }

    pub fn split_vsock(addr: &str) -> Result<VsockAddr, VsockAddrParseError> {
        debug!("Parsing vsock address: {}", addr);

        let (cid, port) = addr
            .split_once(':')
            .ok_or(VsockAddrParseError::SplitError)?;

        debug!("Split vsock address into cid='{}' and port='{}'", cid, port);

        let cid = cid.parse().map_err(VsockAddrParseError::CidParseError)?;
        let port = port.parse().map_err(VsockAddrParseError::PortParseError)?;

        debug!("Parsed vsock address: cid={}, port={}", cid, port);

        Ok(VsockAddr::new(cid, port))
    }
}
