pub mod cli;
pub mod cli_parser;
pub mod config;
pub mod cats;
pub mod vsock;

use cli_parser::{CommandOutput, DirArgs, FileArgs, ListenArgs, RunArgs};
use crate::config::AppConfig;
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
        VsockSocket { socket_fd }
    }
}

impl Drop for VsockSocket {
    fn drop(&mut self) {
        shutdown(self.socket_fd, Shutdown::Both)
            .unwrap_or_else(|e| eprintln!("Failed to shut socket down: {:?}", e));
        close(self.socket_fd).unwrap_or_else(|e| eprintln!("Failed to close socket: {:?}", e));
    }
}

impl AsRawFd for VsockSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.socket_fd
    }
}

fn vsock_connect(cid: u32, port: u32) -> Result<VsockSocket, String> {
    let sockaddr = VsockAddr::new(cid, port);
    let mut err_msg = String::new();

    for i in 0..MAX_CONNECTION_ATTEMPTS {
        let vsocket = VsockSocket::new(
            socket(
                AddressFamily::Vsock,
                SockType::Stream,
                SockFlag::empty(),
                None,
            )
            .map_err(|err| format!("Failed to create the socket: {:?}", err))?,
        );
        match connect(vsocket.as_raw_fd(), &sockaddr) {
            Ok(_) => return Ok(vsocket),
            Err(e) => err_msg = format!("Failed to connect: {}", e),
        }

        std::thread::sleep(std::time::Duration::from_secs(1 << i));
    }

    Err(err_msg)
}

// The server-side (residential) functions for trusted enclave part

fn run_cmd_server(fd: RawFd, no_wait: bool, _app_config: &AppConfig) -> Result<(), String> {
    // recv command
    let len = recv_u64(fd)?;
    let mut buf = [0u8; BUF_MAX_LEN_CMD];
    recv_loop(fd, &mut buf, len)?;

    let len_usize = len.try_into().map_err(|err| format!("{:?}", err))?;
    let command = std::str::from_utf8(&buf[0..len_usize]).map_err(|err| format!("{:?}", err))?;

    // execute command
    let command_output = if no_wait {
        #[rustfmt::skip]
        let output = Command::new("bash")
            .arg("-c")
            .arg("--")
            .arg(command)
            .spawn();
        if output.is_err() {
            CommandOutput::new(
                String::new(),
                format!("Could not execute the command {}", command),
                1,
            )
        } else {
            CommandOutput::new(String::new(), String::new(), 0)
        }
    } else {
        let output = Command::new("bash")
            .arg("-c")
            .arg("--")
            .arg(command)
            .output()
            .map_err(|err| format!("Could not execute the command {}: {:?}", command, err))?;
        CommandOutput::new_from(output)?
    };

    // send output
    let json_output = serde_json::to_string(&command_output)
        .map_err(|err| format!("Could not serialize the output: {:?}", err))?;
    let buf = json_output.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
    send_u64(fd, len)?;
    send_loop(fd, buf, len)?;
    Ok(())
}

fn send_file_server(fd: RawFd, _app_config: &AppConfig) -> Result<(), String> {
    // recv file path
    let len = recv_u64(fd)?;
    let mut path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
    recv_loop(fd, &mut path_buf, len)?;
    let len_usize = len.try_into().map_err(|err| format!("{:?}", err))?;
    let path = std::str::from_utf8(&path_buf[0..len_usize]).map_err(|err| format!("{:?}", err))?;

    let mut file = File::open(path).map_err(|err| format!("Could not open file {:?}", err))?;

    // send file size
    let filesize = file
        .metadata()
        .map_err(|err| format!("Could not get file metadata {:?}", err))?
        .len();

    send_u64(fd, filesize)?;
    println!("Sending file {} - size {}", path, filesize);

    // send file
    let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
    let mut progress: u64 = 0;
    let mut tmpsize: u64;

    while progress < filesize {
        tmpsize = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
        tmpsize = min(tmpsize, filesize - progress);

        file.read_exact(&mut buf[..tmpsize.try_into().map_err(|err| format!("{:?}", err))?])
            .map_err(|err| format!("Could not read {:?}", err))?;
        send_loop(fd, &buf, tmpsize)?;
        progress += tmpsize;
        print!("\rFile transmission progress (sending from enclave): {:.3}%", progress as f32 / filesize as f32 * 100.0);
    }
    println!("\nFile transmission (sending from enclave) finished.");

    Ok(())
}

fn recv_file_server(fd: RawFd, _app_config: &AppConfig) -> Result<(), String> {
    // recv file path
    let len = recv_u64(fd)?;
    let mut path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
    recv_loop(fd, &mut path_buf, len)?;
    let len_usize = len.try_into().map_err(|err| format!("{:?}", err))?;
    let path = std::str::from_utf8(&path_buf[0..len_usize]).map_err(|err| format!("{:?}", err))?;

    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).map_err(|err| format!("Could not create directories {:?}", err))?;
    }

    let mut file = File::create(path).map_err(|err| format!("Could not open file {:?}", err))?;

    // receive filesize
    let filesize = recv_u64(fd)?;
    println!("Receiving file {} - size {}", path, filesize);

    // receive file
    let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
    let mut progress: u64 = 0;
    let mut tmpsize: u64;

    while progress < filesize {
        tmpsize = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
        tmpsize = min(tmpsize, filesize - progress);

        recv_loop(fd, &mut buf, tmpsize)?;
        file.write_all(&buf[..tmpsize.try_into().map_err(|err| format!("{:?}", err))?])
            .map_err(|err| format!("Could not write {:?}", err))?;
        progress += tmpsize;
        print!("\rFile transmission progress (receiving into enclave): {:.3}%", progress as f32 / filesize as f32 * 100.0);
    }
    println!("\nFile transmission (receiving into enclave) finished.");

    Ok(())
}

/// Helper function to collect all files in a directory recursively
fn collect_files_recursively(path: &Path, base_path: &Path, files: &mut Vec<(String, String)>) -> Result<(), String> {
    if path.is_dir() {
        let entries = fs::read_dir(path)
            .map_err(|err| format!("Could not read directory {:?}: {:?}", path, err))?;

        for entry in entries {
            let entry = entry.map_err(|err| format!("Could not read entry: {:?}", err))?;
            let entry_path = entry.path();
            collect_files_recursively(&entry_path, base_path, files)?;
        }
    } else if path.is_file() {
        let relative_path = path.strip_prefix(base_path)
            .map_err(|err| format!("Could not strip prefix: {:?}", err))?;
        let relative_str = relative_path.to_string_lossy().to_string();
        let absolute_str = path.to_string_lossy().to_string();
        files.push((absolute_str, relative_str));
    }
    Ok(())
}

/// Server function to send a directory recursively (enclave -> host)
fn send_dir_server(fd: RawFd, app_config: &AppConfig) -> Result<(), String> {
    // recv remote directory path
    let len = recv_u64(fd)?;
    let mut path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
    recv_loop(fd, &mut path_buf, len)?;
    let len_usize = len.try_into().map_err(|err| format!("{:?}", err))?;
    let remote_dir = std::str::from_utf8(&path_buf[0..len_usize]).map_err(|err| format!("{:?}", err))?;

    let remote_path = Path::new(remote_dir);

    if !remote_path.exists() {
        // Send 0 files count to indicate error
        send_u64(fd, 0)?;
        return Err(format!("Directory does not exist: {}", remote_dir));
    }

    if !remote_path.is_dir() {
        send_u64(fd, 0)?;
        return Err(format!("Path is not a directory: {}", remote_dir));
    }

    // Collect all files recursively
    let mut files: Vec<(String, String)> = Vec::new();
    collect_files_recursively(remote_path, remote_path, &mut files)?;

    // Send number of files
    let file_count: u64 = files.len().try_into().map_err(|err| format!("{:?}", err))?;
    send_u64(fd, file_count)?;
    println!("Sending directory {} with {} files", remote_dir, file_count);

    // Send each file
    for (absolute_path, relative_path) in files {
        // Send relative path
        let path_bytes = relative_path.as_bytes();
        let path_len: u64 = path_bytes.len().try_into().map_err(|err| format!("{:?}", err))?;
        send_u64(fd, path_len)?;
        send_loop(fd, path_bytes, path_len)?;

        // Open and send file
        let mut file = File::open(&absolute_path)
            .map_err(|err| format!("Could not open file {:?}", err))?;

        let filesize = file
            .metadata()
            .map_err(|err| format!("Could not get file metadata {:?}", err))?
            .len();

        send_u64(fd, filesize)?;
        println!("Sending file {} - size {}", relative_path, filesize);

        let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
        let mut progress: u64 = 0;
        let mut tmpsize: u64;

        while progress < filesize {
            tmpsize = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
            tmpsize = min(tmpsize, filesize - progress);

            file.read_exact(&mut buf[..tmpsize.try_into().map_err(|err| format!("{:?}", err))?])
                .map_err(|err| format!("Could not read {:?}", err))?;
            send_loop(fd, &buf, tmpsize)?;
            progress += tmpsize;
        }
        println!("File {} sent.", relative_path);
    }
    println!("\nDirectory transmission (sending from enclave) finished.");

    Ok(())
}

/// Server function to receive a directory recursively (host -> enclave)
fn recv_dir_server(fd: RawFd, app_config: &AppConfig) -> Result<(), String> {
    // recv remote directory path (destination in enclave)
    let len = recv_u64(fd)?;
    let mut path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
    recv_loop(fd, &mut path_buf, len)?;
    let len_usize = len.try_into().map_err(|err| format!("{:?}", err))?;
    let remote_dir = std::str::from_utf8(&path_buf[0..len_usize]).map_err(|err| format!("{:?}", err))?;

    // Create destination directory
    fs::create_dir_all(remote_dir)
        .map_err(|err| format!("Could not create directory {}: {:?}", remote_dir, err))?;

    // Receive number of files
    let file_count = recv_u64(fd)?;
    println!("Receiving directory {} with {} files", remote_dir, file_count);

    // Receive each file
    for _ in 0..file_count {
        // Receive relative path
        let path_len = recv_u64(fd)?;
        let mut rel_path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
        recv_loop(fd, &mut rel_path_buf, path_len)?;
        let path_len_usize: usize = path_len.try_into().map_err(|err| format!("{:?}", err))?;
        let relative_path = std::str::from_utf8(&rel_path_buf[0..path_len_usize])
            .map_err(|err| format!("{:?}", err))?;

        // Construct full path
        let full_path = Path::new(remote_dir).join(relative_path);

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Could not create directories {:?}", err))?;
        }

        // Receive file size
        let filesize = recv_u64(fd)?;
        println!("Receiving file {} - size {}", relative_path, filesize);

        // Create and write file
        let mut file = File::create(&full_path)
            .map_err(|err| format!("Could not create file {:?}", err))?;

        let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
        let mut progress: u64 = 0;
        let mut tmpsize: u64;

        while progress < filesize {
            tmpsize = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
            tmpsize = min(tmpsize, filesize - progress);

            recv_loop(fd, &mut buf, tmpsize)?;
            file.write_all(&buf[..tmpsize.try_into().map_err(|err| format!("{:?}", err))?])
                .map_err(|err| format!("Could not write {:?}", err))?;
            progress += tmpsize;
        }
        println!("File {} received.", relative_path);
    }
    println!("\nDirectory transmission (receiving into enclave) finished.");

    Ok(())
}

pub fn listen(args: ListenArgs, app_config: AppConfig) -> Result<(), String> {
    let socket_fd = socket(
        AddressFamily::Vsock,
        SockType::Stream,
        SockFlag::empty(),
        None,
    )
    .map_err(|err| format!("Create socket failed: {:?}", err))?;

    let sockaddr = VsockAddr::new(VMADDR_CID_ANY, args.port);

    bind(socket_fd, &sockaddr).map_err(|err| format!("Bind failed: {:?}", err))?;

    listen_vsock(socket_fd, BACKLOG).map_err(|err| format!("Listen failed: {:?}", err))?;

    loop {
        let fd = accept(socket_fd).map_err(|err| format!("Accept failed: {:?}", err))?;

        //check command id
        let cmdid = match recv_u64(fd) {
            Ok(id_u64) => match CmdId::from_u64(id_u64) {
                Some(c) => c,
                _ => {
                    eprintln!("Error no such command");
                    continue;
                }
            },
            Err(e) => {
                eprintln!("Error {}", e);
                continue;
            }
        };

        match cmdid {
            CmdId::RunCmd => {
                if let Err(e) = run_cmd_server(fd, false, &app_config) {
                    eprintln!("Error {}", e);
                }
            }
            CmdId::RunCmdNoWait => {
                if let Err(e) = run_cmd_server(fd, true, &app_config) {
                    eprintln!("Error {}", e);
                }
            }
            CmdId::SendFile => {
                if let Err(e) = recv_file_server(fd, &app_config) {
                    eprintln!("Error {}", e);
                }
            }
            CmdId::RecvFile => {
                if let Err(e) = send_file_server(fd, &app_config) {
                    eprintln!("Error {}", e);
                }
            }
            CmdId::SendDir => {
                if let Err(e) = recv_dir_server(fd, &app_config) {
                    eprintln!("Error {}", e);
                }
            }
            CmdId::RecvDir => {
                if let Err(e) = send_dir_server(fd, &app_config) {
                    eprintln!("Error {}", e);
                }
            }
        }
    }
}

// The client-side functions for untrusted host part

pub fn run(args: RunArgs, _app_config: AppConfig) -> Result<i32, String> {
    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    if args.no_wait {
        send_u64(socket_fd, CmdId::RunCmdNoWait as u64)?;
    } else {
        send_u64(socket_fd, CmdId::RunCmd as u64)?;
    }

    // send command
    let buf = args.command.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // recv command output
    let mut buf = [0u8; BUF_MAX_LEN_CMD_IO];
    let len = recv_u64(socket_fd)?;
    let mut json_output = String::new();
    let mut to_recv = len;
    while to_recv > 0 {
        let recv_len = min(BUF_MAX_LEN_CMD_IO as u64, to_recv);
        recv_loop(socket_fd, &mut buf, recv_len)?;
        to_recv -= recv_len;
        let to_recv_usize: usize = recv_len.try_into().map_err(|err| format!("{:?}", err))?;
        json_output.push_str(
            std::str::from_utf8(&buf[0..to_recv_usize]).map_err(|err| format!("{:?}", err))?,
        );
    }

    let output: CommandOutput = serde_json::from_str(json_output.as_str())
        .map_err(|err| format!("Could not deserialize the output: {:?}", err))?;
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);

    Ok(output.rc.unwrap_or_default())
}

pub fn recv_file(args: FileArgs, _app_config: AppConfig) -> Result<(), String> {
    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(&args.localfile).parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Could not create directories {:?}", err))?;
    }

    let mut file = File::create(&args.localfile)
        .map_err(|err| format!("Could not open localfile {:?}", err))?;
    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    send_u64(socket_fd, CmdId::RecvFile as u64)?;

    // send remote file path
    let buf = args.remotefile.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // receive file size
    let filesize = recv_u64(socket_fd)?;
    println!(
        "Receiving file {}(saving to {}) - size {}",
        &args.remotefile,
        &args.localfile[..],
        filesize
    );

    // receive file
    let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
    let mut progress: u64 = 0;
    let mut tmpsize: u64;

    while progress < filesize {
        tmpsize = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
        tmpsize = min(tmpsize, filesize - progress);

        recv_loop(socket_fd, &mut buf, tmpsize)?;
        file.write_all(&buf[..tmpsize.try_into().map_err(|err| format!("{:?}", err))?])
            .map_err(|err| format!("Could not write {:?}", err))?;
        progress += tmpsize;
        print!("\rFile transmission progress (receiving from enclave): {:.3}%", progress as f32 / filesize as f32 * 100.0);
    }
    println!("\nFile transmission (receiving from enclave) finished.");

    Ok(())
}

pub fn send_file(args: FileArgs, _app_config: AppConfig) -> Result<(), String> {
    let mut file =
        File::open(&args.localfile).map_err(|err| format!("Could not open localfile {:?}", err))?;
    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    send_u64(socket_fd, CmdId::SendFile as u64)?;

    // send remote file path
    let buf = args.remotefile.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // send filesize
    let filesize = file
        .metadata()
        .map_err(|err| format!("Could not get file metadate {:?}", err))?
        .len();

    send_u64(socket_fd, filesize)?;
    println!(
        "Sending file {}(sending to {}) - size {}",
        &args.localfile,
        &args.remotefile[..],
        filesize
    );

    // send file
    let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
    let mut progress: u64 = 0;
    let mut tmpsize: u64;

    while progress < filesize {
        tmpsize = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
        tmpsize = min(tmpsize, filesize - progress);

        file.read_exact(&mut buf[..tmpsize.try_into().map_err(|err| format!("{:?}", err))?])
            .map_err(|err| format!("Could not read {:?}", err))?;
        send_loop(socket_fd, &buf, tmpsize)?;
        progress += tmpsize;
        print!("\rFile transmission progress (sending to enclave): {:.3}%", progress as f32 / filesize as f32 * 100.0);
    }
    println!("\nFile transmission (sending to enclave) finished.");

    Ok(())
}

/// Client function to send a directory recursively (host -> enclave)
pub fn send_dir(args: DirArgs, _app_config: AppConfig) -> Result<(), String> {
    let local_path = Path::new(&args.localdir);

    if !local_path.exists() {
        return Err(format!("Local directory does not exist: {}", args.localdir));
    }

    if !local_path.is_dir() {
        return Err(format!("Local path is not a directory: {}", args.localdir));
    }

    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    send_u64(socket_fd, CmdId::SendDir as u64)?;

    // send remote directory path
    let buf = args.remotedir.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // Collect all files recursively
    let mut files: Vec<(String, String)> = Vec::new();
    collect_files_recursively(local_path, local_path, &mut files)?;

    // Send number of files
    let file_count: u64 = files.len().try_into().map_err(|err| format!("{:?}", err))?;
    send_u64(socket_fd, file_count)?;
    println!(
        "Sending directory {}(sending to {}) - {} files",
        &args.localdir,
        &args.remotedir,
        file_count
    );

    // Send each file
    for (absolute_path, relative_path) in files {
        // Send relative path
        let path_bytes = relative_path.as_bytes();
        let path_len: u64 = path_bytes.len().try_into().map_err(|err| format!("{:?}", err))?;
        send_u64(socket_fd, path_len)?;
        send_loop(socket_fd, path_bytes, path_len)?;

        // Open and send file
        let mut file = File::open(&absolute_path)
            .map_err(|err| format!("Could not open file {:?}", err))?;

        let filesize = file
            .metadata()
            .map_err(|err| format!("Could not get file metadata {:?}", err))?
            .len();

        send_u64(socket_fd, filesize)?;
        println!("Sending file {} - size {}", relative_path, filesize);

        let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
        let mut progress: u64 = 0;
        let mut tmpsize: u64;

        while progress < filesize {
            tmpsize = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
            tmpsize = min(tmpsize, filesize - progress);

            file.read_exact(&mut buf[..tmpsize.try_into().map_err(|err| format!("{:?}", err))?])
                .map_err(|err| format!("Could not read {:?}", err))?;
            send_loop(socket_fd, &buf, tmpsize)?;
            progress += tmpsize;
        }
        println!("File {} sent.", relative_path);
    }
    println!("\nDirectory transmission (sending to enclave) finished.");

    Ok(())
}

/// Client function to receive a directory recursively (enclave -> host)
pub fn recv_dir(args: DirArgs, _app_config: AppConfig) -> Result<(), String> {
    // Create local directory
    fs::create_dir_all(&args.localdir)
        .map_err(|err| format!("Could not create local directory {}: {:?}", args.localdir, err))?;

    let vsocket = vsock_connect(args.cid, args.port)?;
    let socket_fd = vsocket.as_raw_fd();

    // send command id
    send_u64(socket_fd, CmdId::RecvDir as u64)?;

    // send remote directory path
    let buf = args.remotedir.as_bytes();
    let len: u64 = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
    send_u64(socket_fd, len)?;
    send_loop(socket_fd, buf, len)?;

    // Receive number of files
    let file_count = recv_u64(socket_fd)?;

    if file_count == 0 {
        return Err(format!("Remote directory is empty or does not exist: {}", args.remotedir));
    }

    println!(
        "Receiving directory {}(saving to {}) - {} files",
        &args.remotedir,
        &args.localdir,
        file_count
    );

    // Receive each file
    for _ in 0..file_count {
        // Receive relative path
        let path_len = recv_u64(socket_fd)?;
        let mut rel_path_buf = [0u8; BUF_MAX_LEN_FILE_PATH];
        recv_loop(socket_fd, &mut rel_path_buf, path_len)?;
        let path_len_usize: usize = path_len.try_into().map_err(|err| format!("{:?}", err))?;
        let relative_path = std::str::from_utf8(&rel_path_buf[0..path_len_usize])
            .map_err(|err| format!("{:?}", err))?;

        // Construct full local path
        let full_path = Path::new(&args.localdir).join(relative_path);

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Could not create directories {:?}", err))?;
        }

        // Receive file size
        let filesize = recv_u64(socket_fd)?;
        println!("Receiving file {} - size {}", relative_path, filesize);

        // Create and write file
        let mut file = File::create(&full_path)
            .map_err(|err| format!("Could not create file {:?}", err))?;

        let mut buf = [0u8; BUF_MAX_LEN_FILE_IO];
        let mut progress: u64 = 0;
        let mut tmpsize: u64;

        while progress < filesize {
            tmpsize = buf.len().try_into().map_err(|err| format!("{:?}", err))?;
            tmpsize = min(tmpsize, filesize - progress);

            recv_loop(socket_fd, &mut buf, tmpsize)?;
            file.write_all(&buf[..tmpsize.try_into().map_err(|err| format!("{:?}", err))?])
                .map_err(|err| format!("Could not write {:?}", err))?;
            progress += tmpsize;
        }
        println!("File {} received.", relative_path);
    }
    println!("\nDirectory transmission (receiving from enclave) finished.");

    Ok(())
}
