//! VSock log streaming implementation for the init system.
//!
//! This module provides the VsockLogStreamer which connects from inside the enclave
//! to a host listener and streams log lines in real-time.

use crate::logger::{Logger, LogSubscriber};
use anyhow::{Context, Result};
use nix::sys::socket::{connect, send, socket, AddressFamily, MsgFlags, SockFlag, SockType, VsockAddr};
use nix::unistd::close;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// VSock log stream subscriber - streams logs to a VSock connection on the host.
///
/// This is used inside the enclave to push log lines to a listener on the host.
pub struct VsockLogStreamer {
    socket_fd: RawFd,
    active: Arc<AtomicBool>,
    service_name: String,
    vsock_cid: u32,
    vsock_port: u32,
}

impl VsockLogStreamer {
    /// Create a new VsockLogStreamer and connect to the specified VSock address.
    ///
    /// # Arguments
    /// * `cid` - The CID of the host to connect to (typically 2 for VMADDR_CID_HOST from enclave)
    /// * `port` - The VSock port to connect to
    /// * `service_name` - Name of the service for logging purposes
    pub fn new(cid: u32, port: u32, service_name: &str) -> Result<Self> {
        let socket_fd = socket(
            AddressFamily::Vsock,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )
        .context("Failed to create VSock socket for log streaming")?;

        let addr = VsockAddr::new(cid, port);
        connect(socket_fd, &addr).context(format!(
            "Failed to connect to VSock CID:{} PORT:{} for log streaming",
            cid, port
        ))?;

        Logger::info(&format!(
            "Log streaming connected for service '{}' to CID:{} PORT:{}",
            service_name, cid, port
        ));

        Ok(Self {
            socket_fd,
            active: Arc::new(AtomicBool::new(true)),
            service_name: service_name.to_string(),
            vsock_cid: cid,
            vsock_port: port,
        })
    }

    /// Stop streaming and close the connection.
    pub fn stop(&self) {
        if self.active.swap(false, Ordering::Relaxed) {
            let _ = close(self.socket_fd);
        }
    }

    /// Get the service name this streamer is associated with.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the VSock CID this streamer is connected to.
    pub fn vsock_cid(&self) -> u32 {
        self.vsock_cid
    }

    /// Get the VSock port this streamer is connected to.
    pub fn vsock_port(&self) -> u32 {
        self.vsock_port
    }
}

impl Drop for VsockLogStreamer {
    fn drop(&mut self) {
        self.stop();
    }
}

impl LogSubscriber for VsockLogStreamer {
    fn on_log(&self, line: &str) {
        if !self.is_active() {
            return;
        }

        // Send log line with newline terminator
        let data = format!("{}\n", line);
        if send(self.socket_fd, data.as_bytes(), MsgFlags::empty()).is_err() {
            // Connection lost, mark as inactive
            self.active.store(false, Ordering::Relaxed);
        }
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streamer_creation_fails_without_listener() {
        // This should fail because there's no listener
        let result = VsockLogStreamer::new(2, 99999, "test-service");
        assert!(result.is_err());
    }
}
