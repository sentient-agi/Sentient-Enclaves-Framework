// Based on:
// https://github.com/linkerd/linkerd2-proxy/blob/release/v2.187.0/linkerd/proxy/transport/src/orig_dst.rs

use std::fmt::Debug;
use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::{debug, info};

pub trait AddrInfo: Debug {
    fn local_addr(&self) -> Result<SocketAddr, io::Error>;
    fn get_original_dst(&self) -> Option<SocketAddr>;
}

impl<T: AddrInfo + ?Sized> AddrInfo for Box<T> {
    fn local_addr(&self) -> Result<SocketAddr, io::Error> {
        self.as_ref().local_addr()
    }

    fn get_original_dst(&self) -> Option<SocketAddr> {
        self.as_ref().get_original_dst()
    }
}

impl AddrInfo for TcpStream {
    fn local_addr(&self) -> Result<SocketAddr, io::Error> {
        TcpStream::local_addr(self)
    }

    #[cfg(target_os = "linux")]
    fn get_original_dst(&self) -> Option<SocketAddr> {
        use std::os::unix::io::AsRawFd;

        let fd = self.as_raw_fd();
        debug!("Attempting to get original destination from fd={}", fd);
        let ret = unsafe { linux::so_original_dst(fd) };
        match &ret {
            Ok(addr) => debug!("Successfully retrieved original destination: {:?}", addr),
            Err(e) => debug!("Failed to retrieve original destination: {:?}", e),
        }
        ret.ok()
    }

    #[cfg(not(target_os = "linux"))]
    fn get_original_dst(&self) -> Option<SocketAddr> {
        info!("Non Linux system, no support for SO_ORIGINAL_DST");
        None
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
    use std::os::unix::io::RawFd;
    use std::{io, mem};
    use tracing::{debug, error};

    pub unsafe fn so_original_dst(fd: RawFd) -> io::Result<SocketAddr> {
        debug!("Calling getsockopt with SO_ORIGINAL_DST on fd={}", fd);

        let mut sockaddr: libc::sockaddr_storage = mem::zeroed();
        let mut socklen: libc::socklen_t = mem::size_of::<libc::sockaddr_storage>() as u32;

        let ret = libc::getsockopt(
            fd,
            libc::SOL_IP,
            libc::SO_ORIGINAL_DST,
            &mut sockaddr as *mut _ as *mut _,
            &mut socklen as *mut _ as *mut _,
        );
        if ret != 0 {
            let e = io::Error::last_os_error();
            error!("Failed to read SO_ORIGINAL_DST: {:?}", e);
            return Err(e);
        }

        debug!("getsockopt succeeded, parsing socket address with socklen={}", socklen);
        mk_addr(&sockaddr, socklen)
    }

    // Borrowed with love from net2-rs
    // https://github.com/rust-lang-nursery/net2-rs/blob/1b4cb4fb05fbad750b271f38221eab583b666e5e/src/socket.rs#L103
    fn mk_addr(storage: &libc::sockaddr_storage, len: libc::socklen_t) -> io::Result<SocketAddr> {
        match storage.ss_family as libc::c_int {
            libc::AF_INET => {
                debug!("Parsing AF_INET address");
                assert!(len as usize >= mem::size_of::<libc::sockaddr_in>());

                let sa = {
                    let sa = storage as *const _ as *const libc::sockaddr_in;
                    unsafe { *sa }
                };

                let bits = ntoh32(sa.sin_addr.s_addr);
                let ip = Ipv4Addr::new(
                    (bits >> 24) as u8,
                    (bits >> 16) as u8,
                    (bits >> 8) as u8,
                    bits as u8,
                );
                let port = sa.sin_port;
                let addr = SocketAddr::V4(SocketAddrV4::new(ip, ntoh16(port)));
                debug!("Parsed IPv4 address: {:?}", addr);
                Ok(addr)
            }
            libc::AF_INET6 => {
                debug!("Parsing AF_INET6 address");
                assert!(len as usize >= mem::size_of::<libc::sockaddr_in6>());

                let sa = {
                    let sa = storage as *const _ as *const libc::sockaddr_in6;
                    unsafe { *sa }
                };

                let arr = sa.sin6_addr.s6_addr;
                let ip = Ipv6Addr::new(
                    (arr[0] as u16) << 8 | (arr[1] as u16),
                    (arr[2] as u16) << 8 | (arr[3] as u16),
                    (arr[4] as u16) << 8 | (arr[5] as u16),
                    (arr[6] as u16) << 8 | (arr[7] as u16),
                    (arr[8] as u16) << 8 | (arr[9] as u16),
                    (arr[10] as u16) << 8 | (arr[11] as u16),
                    (arr[12] as u16) << 8 | (arr[13] as u16),
                    (arr[14] as u16) << 8 | (arr[15] as u16),
                );

                let port = sa.sin6_port;
                let flowinfo = sa.sin6_flowinfo;
                let scope_id = sa.sin6_scope_id;
                let addr = SocketAddr::V6(SocketAddrV6::new(
                    ip,
                    ntoh16(port),
                    flowinfo,
                    scope_id,
                ));
                debug!("Parsed IPv6 address: {:?}", addr);
                Ok(addr)
            }
            other => {
                error!("Invalid address family: {}", other);
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid argument",
                ))
            }
        }
    }

    fn ntoh16(i: u16) -> u16 {
        <u16>::from_be(i)
    }

    fn ntoh32(i: u32) -> u32 {
        <u32>::from_be(i)
    }
}
