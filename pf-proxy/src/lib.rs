pub mod addr_info;

pub mod utils {
    use std::num::ParseIntError;

    use tokio_vsock::VsockAddr;

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
        let (cid, port) = addr
            .split_once(':')
            .ok_or(VsockAddrParseError::SplitError)?;
        let cid = cid.parse().map_err(VsockAddrParseError::CidParseError)?;
        let port = port.parse().map_err(VsockAddrParseError::PortParseError)?;

        Ok(VsockAddr::new(cid, port))
    }
}
