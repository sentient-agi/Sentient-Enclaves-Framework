use byteorder::{ByteOrder, LittleEndian};
use nix::sys::socket::MsgFlags;
use nix::sys::socket::{recv, send};
use std::convert::TryInto;
use std::os::unix::io::RawFd;
use tracing::{debug, error, info};

use crate::error::{PipelineError, Result};

pub fn send_u64(fd: RawFd, val: u64) -> Result<()> {
    debug!(fd = fd, value = val, "Sending u64 value");
    let mut buf = [0u8; 9];
    LittleEndian::write_u64(&mut buf, val);
    send_loop(fd, &buf, 9)?;
    debug!(fd = fd, value = val, "Successfully sent u64 value");
    Ok(())
}

pub fn recv_u64(fd: RawFd) -> Result<u64> {
    debug!(fd = fd, "Receiving u64 value");
    let mut buf = [0u8; 9];
    recv_loop(fd, &mut buf, 9)?;
    let val = LittleEndian::read_u64(&buf);
    debug!(fd = fd, value = val, "Successfully received u64 value");
    Ok(val)
}

pub fn send_i32(fd: RawFd, val: i32) -> Result<()> {
    debug!(fd = fd, value = val, "Sending i32 value");
    let mut buf = [0u8; 4];
    LittleEndian::write_i32(&mut buf, val);
    send_loop(fd, &buf, 4)?;
    debug!(fd = fd, value = val, "Successfully sent i32 value");
    Ok(())
}

pub fn recv_i32(fd: RawFd) -> Result<i32> {
    debug!(fd = fd, "Receiving i32 value");
    let mut buf = [0u8; 4];
    recv_loop(fd, &mut buf, 4)?;
    let val = LittleEndian::read_i32(&buf);
    debug!(fd = fd, value = val, "Successfully received i32 value");
    Ok(val)
}

pub fn send_loop(fd: RawFd, buf: &[u8], len: u64) -> Result<()> {
    let len: usize = len.try_into().map_err(|e| {
        error!(error = %e, "Failed to convert length to usize");
        PipelineError::ConversionError(format!("Failed to convert length {} to usize: {}", len, e))
    })?;
    let mut send_bytes = 0;

    debug!(fd = fd, total_len = len, "Starting send loop");

    while send_bytes < len {
        let size = match send(fd, &buf[send_bytes..len], MsgFlags::empty()) {
            Ok(size) => {
                debug!(fd = fd, bytes_sent = size, "Sent bytes in iteration");
                size
            }
            Err(nix::errno::Errno::EINTR) => {
                debug!(fd = fd, "Send interrupted (EINTR), retrying");
                0
            }
            Err(err) => {
                error!(fd = fd, error = %err, "Send failed");
                return Err(PipelineError::SendError {
                    bytes: len,
                    message: format!("nix send error: {}", err),
                });
            }
        };
        send_bytes += size;
    }

    debug!(fd = fd, total_sent = send_bytes, "Send loop completed");
    Ok(())
}

pub fn recv_loop(fd: RawFd, buf: &mut [u8], len: u64) -> Result<()> {
    let len: usize = len.try_into().map_err(|e| {
        error!(error = %e, "Failed to convert length to usize");
        PipelineError::ConversionError(format!("Failed to convert length {} to usize: {}", len, e))
    })?;
    let mut recv_bytes = 0;

    debug!(fd = fd, total_len = len, "Starting recv loop");

    while recv_bytes < len {
        let size = match recv(fd, &mut buf[recv_bytes..len], MsgFlags::empty()) {
            Ok(size) => {
                debug!(fd = fd, bytes_received = size, "Received bytes in iteration");
                size
            }
            Err(nix::errno::Errno::EINTR) => {
                debug!(fd = fd, "Recv interrupted (EINTR), retrying");
                0
            }
            Err(err) => {
                error!(fd = fd, error = %err, "Recv failed");
                return Err(PipelineError::RecvError {
                    bytes: len,
                    message: format!("nix recv error: {}", err),
                });
            }
        };
        recv_bytes += size;
    }

    debug!(fd = fd, total_received = recv_bytes, "Recv loop completed");
    Ok(())
}
