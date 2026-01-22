pub mod cats;
pub mod cli;
pub mod cli_parser;
pub mod config;
pub mod error;
pub mod vsock;

use cli_parser::{CommandOutput, DirArgs, FileArgs, ListenArgs, RunArgs};
use config::AppConfig;
use error::{PipelineError, Result};
use vsock::{recv_loop, recv_u64, send_loop, send_u64};

use nix::sys::socket::listen as listen_vsock;
use nix::sys::socket::{accept, bind, connect, shutdown, socket};
use nix::sys::socket::{AddressFamily, Shutdown, SockFlag, SockType, VsockAddr};
use nix::unistd::close;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::cmp::min;
use std::convert::TryInto;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::process::Command;
use tracing::{debug, error, info};

pub const VMADDR_CID_ANY: u32 = 0xFFFFFFFF;
// Buffer size is tunable from 8192 and for up to 10485760 bytes == 10 MiBs or more, for best throughput.
// Should be less than stack size. See `ulimit -sS` and `ulimit -sH` for current stack size soft and hard limits, correspondingly.
pub const BUF_MAX_LEN_FILE_IO: usize = 7340032; // Files send and receive buffer.
pub const BUF_MAX_LEN_FILE_PATH: usize = 8192; // Buffer for file path.
pub const BUF_MAX_LEN_CMD: usize = 8192; // Buffer for shell commands.
pub const BUF_MAX_LEN_CMD_IO: usize = 10240; // Buffer for shell commands output to STDOUT.
pub const BACKLOG: usize = 128;
pub const MAX_CONNECTION_ATTEMPTS: usize = 10;

#[derive(Debug, Clone, FromPrimitive)]
enum CmdId {
    RunCmd = 0,
    RecvFile,
    SendFile,
    RunCmdNoWait,
    SendDir,
    RecvDir,
}

struct VsockSocket {
    socket_fd: RawFd,
}

impl VsockSocket {
    fn new(socket_fd: RawFd) -> Self {
        debug!(fd = socket_fd, "Creating new VsockSocket");
        VsockSocket { socket_fd }
    }
}

impl Drop for VsockSocket {
    fn drop(&mut self) {
        debug!(fd = self.socket_fd, "Dropping VsockSocket");
        if let Err(e) = shutdown(self.socket_fd, Shutdown::Both) {
            debug!(fd = self.socket_fd, error = %e, "Failed to shut socket down (may already be closed)");
        }
        if let Err(e) = close(self.socket_fd) {
            debug!(fd = self.socket_fd, error = %e, "Failed to close socket (may already be closed)");
        }
        debug!(fd = self.socket_fd, "VsockSocket dropped");
    }
}

impl AsRawFd for VsockSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.socket_fd
    }
}

fn vsock_connect(cid: u32, port: u32) -> Result<VsockSocket> {
    info!(cid = cid, port = port, "Attempting vsock connection");
    let sockaddr = VsockAddr::new(cid, port);
    let mut err_msg = String::new();

    for i in 0..MAX_CONNECTION_ATTEMPTS {
        debug!(attempt = i + 1, max_attempts = MAX_CONNECTION_ATTEMPTS, "Connection attempt");

        let vsocket = VsockSocket::new(
            socket(
                AddressFamily::Vsock,
                SockType::Stream,
                SockFlag::empty(),
                None,
            )
            .map_err(|err| {
                error!(error = %err, "Failed to create the socket");
                PipelineError::SocketError(format!("Failed to create the socket: {}", err))
            })?,
        );

        match connect(vsocket.as_raw_fd(), &sockaddr) {
            Ok(_) => {
                info!(cid = cid, port = port, fd = vsocket.as_raw_fd(), "Successfully connected");
                return Ok(vsocket);
            }
            Err(e) => {
                err_msg = format!("Failed to connect: {}", e);
                debug!(error = %e, attempt = i + 1, "Connection attempt failed, retrying");
            }
        }

        let sleep_duration = std::time::Duration::from_secs(1 << i);
        debug!(duration_secs = sleep_duration.as_secs(), "Sleeping before retry");
        std::thread::sleep(sleep_duration);
    }

    error!(cid = cid, port = port, error = %err_msg, "All connection attempts failed");
    Err(PipelineError::ConnectionError(err_msg))
}

// The server-side (residential) functions for trusted enclave part

fn run_cmd_server(fd: RawFd, no_wait: bool, _app_config: &AppConfig) -> Result<()> {
    debug!(fd = fd, no_wait = no_wait, "Running command server");

    // recv command
    let len = recv_u64(fd)?;
    let mut buf = [0u8; BUF_MAX_LEN_CMD];
    recv_loop(fd, &mut buf, len)?;

    let len_usize: usize = len.try_into().map_err(|e| {
        error!(error = %e, len = len, "Failed to convert length to usize");
        PipelineError::ConversionError(format!("Failed to convert length {} to usize: {}", len, e))
    })?;

    let command = std::str::from_utf8(&buf[0..len_usize]).map_err(|e| {
        error!(error = %e, "Failed to parse command as UTF-8");
        PipelineError::Utf8Error(e)
    })?;

    info!(command = %command, no_wait = no_wait, "Executing command");

    // execute command
    let command_output = if no_wait {
        #[rustfmt::skip]
        let output = Command::new("bash")
            .arg("-c")
            .arg("--")
            .arg(command)
            .spawn();
        if let Err(e) = output {
            error!(command = %command, error = %e, "Failed to spawn command");
            CommandOutput::new(
                String::new(),
                format!("Could not execute the command {}: {}", command, e),
                1,
            )
        } else {
            info!(command = %command, "Command spawned successfully (no-wait mode)");
            CommandOutput::new(String::new(), String::new(), 0)
        }
    } else {
        let output = Command::new("bash")
            .arg("-c")
            .arg("--")
            .arg(command)
            .output()
            .map_err(|err| {
                error!(command = %command, error = %err, "Failed to execute command");
                PipelineError::CommandError(format!("Could not execute the command {}: {}", command, err))
            })?;
        info!(command = %command, status = ?output.status, "Command executed");
        CommandOutput::new_from(output)?
    };

    // send output
    let json_output = serde_json::to_string(&command_output).map_err(|err| {
        error!(error = %err, "Failed to serialize command output");
        PipelineError::SerializationError(format!("Could not serialize the output: {}", err))
    })?;

    let buf = json_output.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|e| {
        error!(error = %e, "Failed to convert buffer length to u64");
        PipelineError::ConversionError(format!("Failed to convert buffer length to u64: {}", e))
    })?;

    send_u64(fd, len)?;
    send_loop(fd, buf, len)?;

    debug!(fd = fd, "Command server completed");
    Ok(())
}

fn send_file_server(fd: RawFd, _app_config: &AppConfig) -> Result<()> {
    debug!(fd = fd, "Starting send file server");

    // recv file path
    let len = recv_u64(fd)?;
    let mut path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
    recv_loop(fd, &mut path_buf, len)?;

    let len_usize: usize = len.try_into().map_err(|e| {
        error!(error = %e, "Failed to convert length to usize");
        PipelineError::ConversionError(format!("Failed to convert length to usize: {}", e))
    })?;

    let path = std::str::from_utf8(&path_buf[0..len_usize]).map_err(|e| {
        error!(error = %e, "Failed to parse path as UTF-8");
        PipelineError::Utf8Error(e)
    })?;

    debug!(path = %path, "Opening file for sending");
    let mut file = File::open(path).map_err(|err| {
        error!(path = %path, error = %err, "Failed to open file");
        PipelineError::FileError {
            operation: "open".to_string(),
            path: path.to_string(),
            message: err.to_string(),
        }
    })?;

    // send file size
    let filesize = file
        .metadata()
        .map_err(|err| {
            error!(path = %path, error = %err, "Failed to get file metadata");
            PipelineError::FileError {
                operation: "metadata".to_string(),
                path: path.to_string(),
                message: err.to_string(),
            }
        })?
        .len();

    send_u64(fd, filesize)?;
    info!(path = %path, size = filesize, "Sending file from enclave");

    // send file
    let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
    let mut progress: u64 = 0;
    let mut tmpsize: u64;

    while progress < filesize {
        tmpsize = buf.len().try_into().map_err(|e| {
            error!(error = %e, "Failed to convert buffer length to u64");
            PipelineError::ConversionError(format!("Failed to convert buffer length to u64: {}", e))
        })?;
        tmpsize = min(tmpsize, filesize - progress);

        let tmpsize_usize: usize = tmpsize.try_into().map_err(|e| {
            error!(error = %e, "Failed to convert tmpsize to usize");
            PipelineError::ConversionError(format!("Failed to convert tmpsize to usize: {}", e))
        })?;

        file.read_exact(&mut buf[..tmpsize_usize]).map_err(|err| {
            error!(path = %path, error = %err, "Failed to read from file");
            PipelineError::FileError {
                operation: "read".to_string(),
                path: path.to_string(),
                message: err.to_string(),
            }
        })?;

        send_loop(fd, &buf, tmpsize)?;
        progress += tmpsize;

        let percent = progress as f32 / filesize as f32 * 100.0;
        debug!(path = %path, progress = progress, total = filesize, percent = format!("{:.3}%", percent), "File transmission progress (sending from enclave)");
    }

    info!(path = %path, size = filesize, "File transmission from enclave finished");
    Ok(())
}

fn recv_file_server(fd: RawFd, _app_config: &AppConfig) -> Result<()> {
    debug!(fd = fd, "Starting receive file server");

    // recv file path
    let len = recv_u64(fd)?;
    let mut path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
    recv_loop(fd, &mut path_buf, len)?;

    let len_usize: usize = len.try_into().map_err(|e| {
        error!(error = %e, "Failed to convert length to usize");
        PipelineError::ConversionError(format!("Failed to convert length to usize: {}", e))
    })?;

    let path = std::str::from_utf8(&path_buf[0..len_usize]).map_err(|e| {
        error!(error = %e, "Failed to parse path as UTF-8");
        PipelineError::Utf8Error(e)
    })?;

    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(path).parent() {
        debug!(parent = %parent.display(), "Creating parent directories");
        fs::create_dir_all(parent).map_err(|err| {
            error!(path = %parent.display(), error = %err, "Failed to create parent directories");
            PipelineError::DirectoryError {
                operation: "create".to_string(),
                path: parent.display().to_string(),
                message: err.to_string(),
            }
        })?;
    }

    debug!(path = %path, "Creating file for receiving");
    let mut file = File::create(path).map_err(|err| {
        error!(path = %path, error = %err, "Failed to create file");
        PipelineError::FileError {
            operation: "create".to_string(),
            path: path.to_string(),
            message: err.to_string(),
        }
    })?;

    // receive filesize
    let filesize = recv_u64(fd)?;
    info!(path = %path, size = filesize, "Receiving file into enclave");

    // receive file
    let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
    let mut progress: u64 = 0;
    let mut tmpsize: u64;

    while progress < filesize {
        tmpsize = buf.len().try_into().map_err(|e| {
            error!(error = %e, "Failed to convert buffer length to u64");
            PipelineError::ConversionError(format!("Failed to convert buffer length to u64: {}", e))
        })?;
        tmpsize = min(tmpsize, filesize - progress);

        recv_loop(fd, &mut buf, tmpsize)?;

        let tmpsize_usize: usize = tmpsize.try_into().map_err(|e| {
            error!(error = %e, "Failed to convert tmpsize to usize");
            PipelineError::ConversionError(format!("Failed to convert tmpsize to usize: {}", e))
        })?;

        file.write_all(&buf[..tmpsize_usize]).map_err(|err| {
            error!(path = %path, error = %err, "Failed to write to file");
            PipelineError::FileError {
                operation: "write".to_string(),
                path: path.to_string(),
                message: err.to_string(),
            }
        })?;

        progress += tmpsize;

        let percent = progress as f32 / filesize as f32 * 100.0;
        debug!(path = %path, progress = progress, total = filesize, percent = format!("{:.3}%", percent), "File transmission progress (receiving into enclave)");
    }

    info!(path = %path, size = filesize, "File transmission into enclave finished");
    Ok(())
}

/// Helper function to collect all files in a directory recursively
fn collect_files_recursively(
    path: &Path,
    base_path: &Path,
    files: &mut Vec<(String, String)>,
) -> Result<()> {
    debug!(path = %path.display(), base = %base_path.display(), "Collecting files recursively");

    if path.is_dir() {
        let entries = fs::read_dir(path).map_err(|err| {
            error!(path = %path.display(), error = %err, "Failed to read directory");
            PipelineError::DirectoryError {
                operation: "read".to_string(),
                path: path.display().to_string(),
                message: err.to_string(),
            }
        })?;

        for entry in entries {
            let entry = entry.map_err(|err| {
                error!(error = %err, "Failed to read directory entry");
                PipelineError::DirectoryError {
                    operation: "read entry".to_string(),
                    path: path.display().to_string(),
                    message: err.to_string(),
                }
            })?;
            let entry_path = entry.path();
            collect_files_recursively(&entry_path, base_path, files)?;
        }
    } else if path.is_file() {
        let relative_path = path.strip_prefix(base_path).map_err(|err| {
            error!(path = %path.display(), base = %base_path.display(), error = %err, "Failed to strip prefix");
            PipelineError::FileError {
                operation: "strip prefix".to_string(),
                path: path.display().to_string(),
                message: err.to_string(),
            }
        })?;
        let relative_str = relative_path.to_string_lossy().to_string();
        let absolute_str = path.to_string_lossy().to_string();
        debug!(absolute = %absolute_str, relative = %relative_str, "Found file");
        files.push((absolute_str, relative_str));
    }
    Ok(())
}

/// Server function to send a directory recursively (enclave -> host)
fn send_dir_server(fd: RawFd, _app_config: &AppConfig) -> Result<()> {
    debug!(fd = fd, "Starting send directory server");

    // recv remote directory path
    let len = recv_u64(fd)?;
    let mut path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
    recv_loop(fd, &mut path_buf, len)?;

    let len_usize: usize = len.try_into().map_err(|e| {
        error!(error = %e, "Failed to convert length to usize");
        PipelineError::ConversionError(format!("Failed to convert length to usize: {}", e))
    })?;

    let remote_dir = std::str::from_utf8(&path_buf[0..len_usize]).map_err(|e| {
        error!(error = %e, "Failed to parse directory path as UTF-8");
        PipelineError::Utf8Error(e)
    })?;

    let remote_path = Path::new(remote_dir);

    if !remote_path.exists() {
        // Send 0 files count to indicate error
        send_u64(fd, 0)?;
        error!(path = %remote_dir, "Directory does not exist");
        return Err(PipelineError::DirectoryError {
            operation: "check existence".to_string(),
            path: remote_dir.to_string(),
            message: "Directory does not exist".to_string(),
        });
    }

    if !remote_path.is_dir() {
        send_u64(fd, 0)?;
        error!(path = %remote_dir, "Path is not a directory");
        return Err(PipelineError::DirectoryError {
            operation: "check type".to_string(),
            path: remote_dir.to_string(),
            message: "Path is not a directory".to_string(),
        });
    }

    // Collect all files recursively
    let mut files: Vec<(String, String)> = Vec::new();
    collect_files_recursively(remote_path, remote_path, &mut files)?;

    // Send number of files
    let file_count: u64 = files.len().try_into().map_err(|e| {
        error!(error = %e, "Failed to convert file count to u64");
        PipelineError::ConversionError(format!("Failed to convert file count to u64: {}", e))
    })?;
    send_u64(fd, file_count)?;
    info!(path = %remote_dir, file_count = file_count, "Sending directory from enclave");

    // Send each file
    for (absolute_path, relative_path) in files {
        // Send relative path
        let path_bytes = relative_path.as_bytes();
        let path_len: u64 = path_bytes.len().try_into().map_err(|e| {
            error!(error = %e, "Failed to convert path length to u64");
            PipelineError::ConversionError(format!("Failed to convert path length to u64: {}", e))
        })?;
        send_u64(fd, path_len)?;
        send_loop(fd, path_bytes, path_len)?;

        // Open and send file
        let mut file = File::open(&absolute_path).map_err(|err| {
            error!(path = %absolute_path, error = %err, "Failed to open file");
            PipelineError::FileError {
                operation: "open".to_string(),
                path: absolute_path.clone(),
                message: err.to_string(),
            }
        })?;

        let filesize = file
            .metadata()
            .map_err(|err| {
                error!(path = %absolute_path, error = %err, "Failed to get file metadata");
                PipelineError::FileError {
                    operation: "metadata".to_string(),
                    path: absolute_path.clone(),
                    message: err.to_string(),
                }
            })?
            .len();

        send_u64(fd, filesize)?;
        info!(path = %relative_path, size = filesize, "Sending file");

        let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
        let mut progress: u64 = 0;
        let mut tmpsize: u64;

        while progress < filesize {
            tmpsize = buf.len().try_into().map_err(|e| {
                error!(error = %e, "Failed to convert buffer length to u64");
                PipelineError::ConversionError(format!("Failed to convert buffer length to u64: {}", e))
            })?;
            tmpsize = min(tmpsize, filesize - progress);

            let tmpsize_usize: usize = tmpsize.try_into().map_err(|e| {
                error!(error = %e, "Failed to convert tmpsize to usize");
                PipelineError::ConversionError(format!("Failed to convert tmpsize to usize: {}", e))
            })?;

            file.read_exact(&mut buf[..tmpsize_usize]).map_err(|err| {
                error!(path = %absolute_path, error = %err, "Failed to read from file");
                PipelineError::FileError {
                    operation: "read".to_string(),
                    path: absolute_path.clone(),
                    message: err.to_string(),
                }
            })?;

            send_loop(fd, &buf, tmpsize)?;
            progress += tmpsize;
        }
        info!(path = %relative_path, "File sent");
    }

    info!(path = %remote_dir, "Directory transmission from enclave finished");
    Ok(())
}

/// Server function to receive a directory recursively (host -> enclave)
fn recv_dir_server(fd: RawFd, _app_config: &AppConfig) -> Result<()> {
    debug!(fd = fd, "Starting receive directory server");

    // recv remote directory path (destination in enclave)
    let len = recv_u64(fd)?;
    let mut path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
    recv_loop(fd, &mut path_buf, len)?;

    let len_usize: usize = len.try_into().map_err(|e| {
        error!(error = %e, "Failed to convert length to usize");
        PipelineError::ConversionError(format!("Failed to convert length to usize: {}", e))
    })?;

    let remote_dir = std::str::from_utf8(&path_buf[0..len_usize]).map_err(|e| {
        error!(error = %e, "Failed to parse directory path as UTF-8");
        PipelineError::Utf8Error(e)
    })?;

    // Create destination directory
    debug!(path = %remote_dir, "Creating destination directory");
    fs::create_dir_all(remote_dir).map_err(|err| {
        error!(path = %remote_dir, error = %err, "Failed to create directory");
        PipelineError::DirectoryError {
            operation: "create".to_string(),
            path: remote_dir.to_string(),
            message: err.to_string(),
        }
    })?;

    // Receive number of files
    let file_count = recv_u64(fd)?;
    info!(path = %remote_dir, file_count = file_count, "Receiving directory into enclave");

    // Receive each file
    for i in 0..file_count {
        debug!(file_index = i, total = file_count, "Receiving file");

        // Receive relative path
        let path_len = recv_u64(fd)?;
        let mut rel_path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
        recv_loop(fd, &mut rel_path_buf, path_len)?;

        let path_len_usize: usize = path_len.try_into().map_err(|e| {
            error!(error = %e, "Failed to convert path length to usize");
            PipelineError::ConversionError(format!("Failed to convert path length to usize: {}", e))
        })?;

        let relative_path = std::str::from_utf8(&rel_path_buf[0..path_len_usize]).map_err(|e| {
            error!(error = %e, "Failed to parse relative path as UTF-8");
            PipelineError::Utf8Error(e)
        })?;

        // Construct full path
        let full_path = Path::new(remote_dir).join(relative_path);

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            debug!(parent = %parent.display(), "Creating parent directories");
            fs::create_dir_all(parent).map_err(|err| {
                error!(path = %parent.display(), error = %err, "Failed to create parent directories");
                PipelineError::DirectoryError {
                    operation: "create".to_string(),
                    path: parent.display().to_string(),
                    message: err.to_string(),
                }
            })?;
        }

        // Receive file size
        let filesize = recv_u64(fd)?;
        info!(path = %relative_path, size = filesize, "Receiving file");

        // Create and write file
        let mut file = File::create(&full_path).map_err(|err| {
            error!(path = %full_path.display(), error = %err, "Failed to create file");
            PipelineError::FileError {
                operation: "create".to_string(),
                path: full_path.display().to_string(),
                message: err.to_string(),
            }
        })?;

        let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
        let mut progress: u64 = 0;
        let mut tmpsize: u64;

        while progress < filesize {
            tmpsize = buf.len().try_into().map_err(|e| {
                error!(error = %e, "Failed to convert buffer length to u64");
                PipelineError::ConversionError(format!("Failed to convert buffer length to u64: {}", e))
            })?;
            tmpsize = min(tmpsize, filesize - progress);

            recv_loop(fd, &mut buf, tmpsize)?;

            let tmpsize_usize: usize = tmpsize.try_into().map_err(|e| {
                error!(error = %e, "Failed to convert tmpsize to usize");
                PipelineError::ConversionError(format!("Failed to convert tmpsize to usize: {}", e))
            })?;

            file.write_all(&buf[..tmpsize_usize]).map_err(|err| {
                error!(path = %full_path.display(), error = %err, "Failed to write to file");
                PipelineError::FileError {
                    operation: "write".to_string(),
                    path: full_path.display().to_string(),
                    message: err.to_string(),
                }
            })?;

            progress += tmpsize;
        }
        info!(path = %relative_path, "File received");
    }

    info!(path = %remote_dir, "Directory transmission into enclave finished");
    Ok(())
}

pub fn listen(args: ListenArgs, app_config: AppConfig) -> Result<()> {
    info!(port = args.port, "Starting listener");

    let socket_fd = socket(
        AddressFamily::Vsock,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )
    .map_err(|err| {
        error!(error = %err, "Failed to create socket");
        PipelineError::SocketError(format!("Create socket failed: {}", err))
    })?;

    let sockaddr = VsockAddr::new(VMADDR_CID_ANY, args.port);

    bind(socket_fd, &sockaddr).map_err(|err| {
        error!(error = %err, port = args.port, "Failed to bind socket");
        PipelineError::SocketError(format!("Bind failed: {}", err))
    })?;

    listen_vsock(socket_fd, BACKLOG).map_err(|err| {
        error!(error = %err, "Failed to listen on socket");
        PipelineError::SocketError(format!("Listen failed: {}", err))
    })?;

    info!(port = args.port, "Listening for connections");

    loop {
        let fd = match accept(socket_fd) {
            Ok(fd) => {
                debug!(fd = fd, "Accepted connection");
                fd
            }
            Err(err) => {
                error!(error = %err, "Accept failed");
                continue;
            }
        };

        // check command id
        let cmdid = match recv_u64(fd) {
            Ok(id_u64) => match CmdId::from_u64(id_u64) {
                Some(c) => {
                    debug!(command_id = ?c, "Received command ID");
                    c
                }
                _ => {
                    error!(id = id_u64, "Unknown command ID received");
                    continue;
                }
            },
            Err(e) => {
                error!(error = %e, "Error receiving command ID");
                continue;
            }
        };

        match cmdid {
            CmdId::RunCmd => {
                info!("Processing RunCmd");
                if let Err(e) = run_cmd_server(fd, false, &app_config) {
                    error!(error = %e, "RunCmd failed");
                }
            }
            CmdId::RunCmdNoWait => {
                info!("Processing RunCmdNoWait");
                if let Err(e) = run_cmd_server(fd, true, &app_config) {
                    error!(error = %e, "RunCmdNoWait failed");
                }
            }
            CmdId::SendFile => {
                info!("Processing SendFile (receiving into enclave)");
                if let Err(e) = recv_file_server(fd, &app_config) {
                    error!(error = %e, "SendFile (recv) failed");
                }
            }
            CmdId::RecvFile => {
                info!("Processing RecvFile (sending from enclave)");
                if let Err(e) = send_file_server(fd, &app_config) {
                    error!(error = %e, "RecvFile (send) failed");
                }
            }
            CmdId::SendDir => {
                info!("Processing SendDir (receiving into enclave)");
                if let Err(e) = recv_dir_server(fd, &app_config) {
                    error!(error = %e, "SendDir (recv) failed");
                }
            }
            CmdId::RecvDir => {
                info!("Processing RecvDir (sending from enclave)");
                if let Err(e) = send_dir_server(fd, &app_config) {
                    error!(error = %e, "RecvDir (send) failed");
                }
            }
        }
    }
}

// The client-side functions for untrusted host part

pub fn run(args: RunArgs, _app_config: AppConfig) -> Result<i32> {
    info!(cid = args.cid, port = args.port, command = %args.command, no_wait = args.no_wait, "Running command");

    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    if args.no_wait {
        debug!("Sending RunCmdNoWait command ID");
        send_u64(socket_fd, CmdId::RunCmdNoWait as u64)?;
    } else {
        debug!("Sending RunCmd command ID");
        send_u64(socket_fd, CmdId::RunCmd as u64)?;
    }

    // send command
    let buf = args.command.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|e| {
        error!(error = %e, "Failed to convert command length to u64");
        PipelineError::ConversionError(format!("Failed to convert command length to u64: {}", e))
    })?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // recv command output
    let mut buf = [0u8; BUF_MAX_LEN_CMD_IO];
    let len = recv_u64(socket_fd)?;
    let mut json_output = String::new();
    let mut to_recv = len;

    debug!(total_len = len, "Receiving command output");

    while to_recv > 0 {
        let recv_len = min(BUF_MAX_LEN_CMD_IO as u64, to_recv);
        recv_loop(socket_fd, &mut buf, recv_len)?;
        to_recv -= recv_len;

        let to_recv_usize: usize = recv_len.try_into().map_err(|e| {
            error!(error = %e, "Failed to convert recv_len to usize");
            PipelineError::ConversionError(format!("Failed to convert recv_len to usize: {}", e))
        })?;

        json_output.push_str(std::str::from_utf8(&buf[0..to_recv_usize]).map_err(|e| {
            error!(error = %e, "Failed to parse output as UTF-8");
            PipelineError::Utf8Error(e)
        })?);
    }

    let output: CommandOutput = serde_json::from_str(json_output.as_str()).map_err(|err| {
        error!(error = %err, "Failed to deserialize command output");
        PipelineError::DeserializationError(format!("Could not deserialize the output: {}", err))
    })?;

    // Log stdout and stderr instead of printing
    if !output.stdout.is_empty() {
        info!(stdout = %output.stdout, "Command stdout");
    }
    if !output.stderr.is_empty() {
        info!(stderr = %output.stderr, "Command stderr");
    }

    let rc = output.rc.unwrap_or_default();
    info!(return_code = rc, "Command completed");
    Ok(rc)
}

pub fn recv_file(args: FileArgs, _app_config: AppConfig) -> Result<()> {
    info!(cid = args.cid, port = args.port, local = %args.localfile, remote = %args.remotefile, "Receiving file from enclave");

    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(&args.localfile).parent() {
        debug!(parent = %parent.display(), "Creating parent directories");
        fs::create_dir_all(parent).map_err(|err| {
            error!(path = %parent.display(), error = %err, "Failed to create parent directories");
            PipelineError::DirectoryError {
                operation: "create".to_string(),
                path: parent.display().to_string(),
                message: err.to_string(),
            }
        })?;
    }

    let mut file = File::create(&args.localfile).map_err(|err| {
        error!(path = %args.localfile, error = %err, "Failed to create local file");
        PipelineError::FileError {
            operation: "create".to_string(),
            path: args.localfile.clone(),
            message: err.to_string(),
        }
    })?;

    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    debug!("Sending RecvFile command ID");
    send_u64(socket_fd, CmdId::RecvFile as u64)?;

    // send remote file path
    let buf = args.remotefile.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|e| {
        error!(error = %e, "Failed to convert path length to u64");
        PipelineError::ConversionError(format!("Failed to convert path length to u64: {}", e))
    })?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // receive file size
    let filesize = recv_u64(socket_fd)?;
    info!(
        remote = %args.remotefile,
        local = %args.localfile,
        size = filesize,
        "Receiving file from enclave"
    );

    // receive file
    let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
    let mut progress: u64 = 0;
    let mut tmpsize: u64;

    while progress < filesize {
        tmpsize = buf.len().try_into().map_err(|e| {
            error!(error = %e, "Failed to convert buffer length to u64");
            PipelineError::ConversionError(format!("Failed to convert buffer length to u64: {}", e))
        })?;
        tmpsize = min(tmpsize, filesize - progress);

        recv_loop(socket_fd, &mut buf, tmpsize)?;

        let tmpsize_usize: usize = tmpsize.try_into().map_err(|e| {
            error!(error = %e, "Failed to convert tmpsize to usize");
            PipelineError::ConversionError(format!("Failed to convert tmpsize to usize: {}", e))
        })?;

        file.write_all(&buf[..tmpsize_usize]).map_err(|err| {
            error!(path = %args.localfile, error = %err, "Failed to write to file");
            PipelineError::FileError {
                operation: "write".to_string(),
                path: args.localfile.clone(),
                message: err.to_string(),
            }
        })?;

        progress += tmpsize;

        let percent = progress as f32 / filesize as f32 * 100.0;
        debug!(progress = progress, total = filesize, percent = format!("{:.3}%", percent), "File transmission progress (receiving from enclave)");
    }

    info!(local = %args.localfile, "File transmission from enclave finished");
    Ok(())
}

pub fn send_file(args: FileArgs, _app_config: AppConfig) -> Result<()> {
    info!(cid = args.cid, port = args.port, local = %args.localfile, remote = %args.remotefile, "Sending file to enclave");

    let mut file = File::open(&args.localfile).map_err(|err| {
        error!(path = %args.localfile, error = %err, "Failed to open local file");
        PipelineError::FileError {
            operation: "open".to_string(),
            path: args.localfile.clone(),
            message: err.to_string(),
        }
    })?;

    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    debug!("Sending SendFile command ID");
    send_u64(socket_fd, CmdId::SendFile as u64)?;

    // send remote file path
    let buf = args.remotefile.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|e| {
        error!(error = %e, "Failed to convert path length to u64");
        PipelineError::ConversionError(format!("Failed to convert path length to u64: {}", e))
    })?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // send filesize
    let filesize = file
        .metadata()
        .map_err(|err| {
            error!(path = %args.localfile, error = %err, "Failed to get file metadata");
            PipelineError::FileError {
                operation: "metadata".to_string(),
                path: args.localfile.clone(),
                message: err.to_string(),
            }
        })?
        .len();

    send_u64(socket_fd, filesize)?;
    info!(
        local = %args.localfile,
        remote = %args.remotefile,
        size = filesize,
        "Sending file to enclave"
    );

    // send file
    let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
    let mut progress: u64 = 0;
    let mut tmpsize: u64;

    while progress < filesize {
        tmpsize = buf.len().try_into().map_err(|e| {
            error!(error = %e, "Failed to convert buffer length to u64");
            PipelineError::ConversionError(format!("Failed to convert buffer length to u64: {}", e))
        })?;
        tmpsize = min(tmpsize, filesize - progress);

        let tmpsize_usize: usize = tmpsize.try_into().map_err(|e| {
            error!(error = %e, "Failed to convert tmpsize to usize");
            PipelineError::ConversionError(format!("Failed to convert tmpsize to usize: {}", e))
        })?;

        file.read_exact(&mut buf[..tmpsize_usize]).map_err(|err| {
            error!(path = %args.localfile, error = %err, "Failed to read from file");
            PipelineError::FileError {
                operation: "read".to_string(),
                path: args.localfile.clone(),
                message: err.to_string(),
            }
        })?;

        send_loop(socket_fd, &buf, tmpsize)?;
        progress += tmpsize;

        let percent = progress as f32 / filesize as f32 * 100.0;
        debug!(progress = progress, total = filesize, percent = format!("{:.3}%", percent), "File transmission progress (sending to enclave)");
    }

    info!(remote = %args.remotefile, "File transmission to enclave finished");
    Ok(())
}

/// Client function to send a directory recursively (host -> enclave)
pub fn send_dir(args: DirArgs, _app_config: AppConfig) -> Result<()> {
    info!(cid = args.cid, port = args.port, local = %args.localdir, remote = %args.remotedir, "Sending directory to enclave");

    let local_path = Path::new(&args.localdir);

    if !local_path.exists() {
        error!(path = %args.localdir, "Local directory does not exist");
        return Err(PipelineError::DirectoryError {
            operation: "check existence".to_string(),
            path: args.localdir.clone(),
            message: "Local directory does not exist".to_string(),
        });
    }

    if !local_path.is_dir() {
        error!(path = %args.localdir, "Local path is not a directory");
        return Err(PipelineError::DirectoryError {
            operation: "check type".to_string(),
            path: args.localdir.clone(),
            message: "Local path is not a directory".to_string(),
        });
    }

    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    debug!("Sending SendDir command ID");
    send_u64(socket_fd, CmdId::SendDir as u64)?;

    // send remote directory path
    let buf = args.remotedir.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|e| {
        error!(error = %e, "Failed to convert path length to u64");
        PipelineError::ConversionError(format!("Failed to convert path length to u64: {}", e))
    })?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // Collect all files recursively
    let mut files: Vec<(String, String)> = Vec::new();
    collect_files_recursively(local_path, local_path, &mut files)?;

    // Send number of files
    let file_count: u64 = files.len().try_into().map_err(|e| {
        error!(error = %e, "Failed to convert file count to u64");
        PipelineError::ConversionError(format!("Failed to convert file count to u64: {}", e))
    })?;
    send_u64(socket_fd, file_count)?;
    info!(
        local = %args.localdir,
        remote = %args.remotedir,
        file_count = file_count,
        "Sending directory to enclave"
    );

    // Send each file
    for (absolute_path, relative_path) in files {
        // Send relative path
        let path_bytes = relative_path.as_bytes();
        let path_len: u64 = path_bytes.len().try_into().map_err(|e| {
            error!(error = %e, "Failed to convert path length to u64");
            PipelineError::ConversionError(format!("Failed to convert path length to u64: {}", e))
        })?;
        send_u64(socket_fd, path_len)?;
        send_loop(socket_fd, path_bytes, path_len)?;

        // Open and send file
        let mut file = File::open(&absolute_path).map_err(|err| {
            error!(path = %absolute_path, error = %err, "Failed to open file");
            PipelineError::FileError {
                operation: "open".to_string(),
                path: absolute_path.clone(),
                message: err.to_string(),
            }
        })?;

        let filesize = file
            .metadata()
            .map_err(|err| {
                error!(path = %absolute_path, error = %err, "Failed to get file metadata");
                PipelineError::FileError {
                    operation: "metadata".to_string(),
                    path: absolute_path.clone(),
                    message: err.to_string(),
                }
            })?
            .len();

        send_u64(socket_fd, filesize)?;
        info!(path = %relative_path, size = filesize, "Sending file");

        let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
        let mut progress: u64 = 0;
        let mut tmpsize: u64;

        while progress < filesize {
            tmpsize = buf.len().try_into().map_err(|e| {
                error!(error = %e, "Failed to convert buffer length to u64");
                PipelineError::ConversionError(format!("Failed to convert buffer length to u64: {}", e))
            })?;
            tmpsize = min(tmpsize, filesize - progress);

            let tmpsize_usize: usize = tmpsize.try_into().map_err(|e| {
                error!(error = %e, "Failed to convert tmpsize to usize");
                PipelineError::ConversionError(format!("Failed to convert tmpsize to usize: {}", e))
            })?;

            file.read_exact(&mut buf[..tmpsize_usize]).map_err(|err| {
                error!(path = %absolute_path, error = %err, "Failed to read from file");
                PipelineError::FileError {
                    operation: "read".to_string(),
                    path: absolute_path.clone(),
                    message: err.to_string(),
                }
            })?;

            send_loop(socket_fd, &buf, tmpsize)?;
            progress += tmpsize;
        }
        info!(path = %relative_path, "File sent");
    }

    info!(remote = %args.remotedir, "Directory transmission to enclave finished");
    Ok(())
}

/// Client function to receive a directory recursively (enclave -> host)
pub fn recv_dir(args: DirArgs, _app_config: AppConfig) -> Result<()> {
    info!(cid = args.cid, port = args.port, local = %args.localdir, remote = %args.remotedir, "Receiving directory from enclave");

    // Create local directory
    debug!(path = %args.localdir, "Creating local directory");
    fs::create_dir_all(&args.localdir).map_err(|err| {
        error!(path = %args.localdir, error = %err, "Failed to create local directory");
        PipelineError::DirectoryError {
            operation: "create".to_string(),
            path: args.localdir.clone(),
            message: err.to_string(),
        }
    })?;

    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    debug!("Sending RecvDir command ID");
    send_u64(socket_fd, CmdId::RecvDir as u64)?;

    // send remote directory path
    let buf = args.remotedir.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|e| {
        error!(error = %e, "Failed to convert path length to u64");
        PipelineError::ConversionError(format!("Failed to convert path length to u64: {}", e))
    })?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // Receive number of files
    let file_count = recv_u64(socket_fd)?;

    if file_count == 0 {
        error!(path = %args.remotedir, "Remote directory is empty or does not exist");
        return Err(PipelineError::DirectoryError {
            operation: "check".to_string(),
            path: args.remotedir.clone(),
            message: "Remote directory is empty or does not exist".to_string(),
        });
    }

    info!(
        remote = %args.remotedir,
        local = %args.localdir,
        file_count = file_count,
        "Receiving directory from enclave"
    );

    // Receive each file
    for i in 0..file_count {
        debug!(file_index = i, total = file_count, "Receiving file");

        // Receive relative path
        let path_len = recv_u64(socket_fd)?;
        let mut rel_path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
        recv_loop(socket_fd, &mut rel_path_buf, path_len)?;

        let path_len_usize: usize = path_len.try_into().map_err(|e| {
            error!(error = %e, "Failed to convert path length to usize");
            PipelineError::ConversionError(format!("Failed to convert path length to usize: {}", e))
        })?;

        let relative_path = std::str::from_utf8(&rel_path_buf[0..path_len_usize]).map_err(|e| {
            error!(error = %e, "Failed to parse relative path as UTF-8");
            PipelineError::Utf8Error(e)
        })?;

        // Construct full local path
        let full_path = Path::new(&args.localdir).join(relative_path);

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            debug!(parent = %parent.display(), "Creating parent directories");
            fs::create_dir_all(parent).map_err(|err| {
                error!(path = %parent.display(), error = %err, "Failed to create parent directories");
                PipelineError::DirectoryError {
                    operation: "create".to_string(),
                    path: parent.display().to_string(),
                    message: err.to_string(),
                }
            })?;
        }

        // Receive file size
        let filesize = recv_u64(socket_fd)?;
        info!(path = %relative_path, size = filesize, "Receiving file");

        // Create and write file
        let mut file = File::create(&full_path).map_err(|err| {
            error!(path = %full_path.display(), error = %err, "Failed to create file");
            PipelineError::FileError {
                operation: "create".to_string(),
                path: full_path.display().to_string(),
                message: err.to_string(),
            }
        })?;

        let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
        let mut progress: u64 = 0;
        let mut tmpsize: u64;

        while progress < filesize {
            tmpsize = buf.len().try_into().map_err(|e| {
                error!(error = %e, "Failed to convert buffer length to u64");
                PipelineError::ConversionError(format!("Failed to convert buffer length to u64: {}", e))
            })?;
            tmpsize = min(tmpsize, filesize - progress);

            recv_loop(socket_fd, &mut buf, tmpsize)?;

            let tmpsize_usize: usize = tmpsize.try_into().map_err(|e| {
                error!(error = %e, "Failed to convert tmpsize to usize");
                PipelineError::ConversionError(format!("Failed to convert tmpsize to usize: {}", e))
            })?;

            file.write_all(&buf[..tmpsize_usize]).map_err(|err| {
                error!(path = %full_path.display(), error = %err, "Failed to write to file");
                PipelineError::FileError {
                    operation: "write".to_string(),
                    path: full_path.display().to_string(),
                    message: err.to_string(),
                }
            })?;

            progress += tmpsize;
        }
        info!(path = %relative_path, "File received");
    }

    info!(local = %args.localdir, "Directory transmission from enclave finished");
    Ok(())
}
