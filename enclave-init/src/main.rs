use anyhow::{Context, Result};
use nix::errno::Errno;
use nix::mount::{mount, MsFlags};
use nix::sys::signal::{
    kill, sigaction, sigprocmask, SaFlags, SigAction, SigHandler, SigSet, SigmaskHow, Signal,
};
use nix::sys::socket::{connect, socket, AddressFamily, SockFlag, SockType, VsockAddr};
use nix::sys::stat::{makedev, mknod, Mode, SFlag};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{
    chdir, chroot, close, fork, read, setsid, setpgid, symlinkat, unlink, write, ForkResult, Pid,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::{self, create_dir, read_dir, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// Constants
const DEFAULT_PATH_ENV: &str = "PATH=/sbin:/usr/sbin:/bin:/usr/bin";
const NSM_PATH: &str = "nsm.ko";
const VSOCK_PORT: u32 = 9000;
const VSOCK_CID: u32 = 3;
const HEART_BEAT: u8 = 0xB7;
const SERVICE_DIR: &str = "/service";

// Global flag for signal handling
static SIGCHLD_RECEIVED: AtomicBool = AtomicBool::new(false);
static SIGTERM_RECEIVED: AtomicBool = AtomicBool::new(false);
static SIGINT_RECEIVED: AtomicBool = AtomicBool::new(false);

// Service configuration
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ServiceConfig {
    #[serde(default)]
    exec_start: String,
    #[serde(default)]
    environment: Vec<String>,
    #[serde(default = "default_restart")]
    restart: RestartPolicy,
    #[serde(default = "default_restart_sec")]
    restart_sec: u64,
    #[serde(default)]
    working_directory: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum RestartPolicy {
    No,
    Always,
    OnFailure,
    OnSuccess,
}

fn default_restart() -> RestartPolicy {
    RestartPolicy::No
}

fn default_restart_sec() -> u64 {
    5
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            exec_start: String::new(),
            environment: Vec::new(),
            restart: RestartPolicy::No,
            restart_sec: 5,
            working_directory: None,
        }
    }
}

// Service state tracking
#[derive(Debug)]
struct ServiceState {
    config: ServiceConfig,
    pid: Option<Pid>,
    name: String,
    restart_count: u32,
    last_restart: Option<Instant>,
    exit_status: Option<i32>,
}

impl ServiceState {
    fn new(name: String, config: ServiceConfig) -> Self {
        Self {
            config,
            pid: None,
            name,
            restart_count: 0,
            last_restart: None,
            exit_status: None,
        }
    }

    fn should_restart(&self, exit_code: i32) -> bool {
        match self.config.restart {
            RestartPolicy::Always => true,
            RestartPolicy::OnFailure => exit_code != 0,
            RestartPolicy::OnSuccess => exit_code == 0,
            RestartPolicy::No => false,
        }
    }

    fn can_restart_now(&self) -> bool {
        if let Some(last) = self.last_restart {
            last.elapsed() >= Duration::from_secs(self.config.restart_sec)
        } else {
            true
        }
    }
}

// Logging helpers
struct Logger;

impl Logger {
    fn init() {
        // Simple stderr logger since we're init
    }

    fn info(msg: &str) {
        eprintln!("[INFO] {}", msg);
    }

    fn warn(msg: &str) {
        eprintln!("[WARN] {}", msg);
    }

    fn error(msg: &str) {
        eprintln!("[ERROR] {}", msg);
    }

    fn debug(msg: &str) {
        eprintln!("[DEBUG] {}", msg);
    }
}

// Operation types
#[derive(Debug)]
enum InitOp {
    Mount {
        source: &'static str,
        target: &'static str,
        fstype: &'static str,
        flags: MsFlags,
        data: Option<&'static str>,
    },
    Mkdir {
        path: &'static str,
        mode: u32,
    },
    Mknod {
        path: &'static str,
        mode: Mode,
        major: u64,
        minor: u64,
    },
    Symlink {
        linkpath: &'static str,
        target: &'static str,
    },
}

// List of initialization operations
// const OPS: std::sync::LazyLock<Vec<InitOp>> = std::sync::LazyLock::new(|| vec![
static OPS: std::sync::LazyLock<Vec<InitOp>> = std::sync::LazyLock::new(|| vec![
    // mount /proc (which should already exist)
    InitOp::Mount {
        source: "proc",
        target: "/proc",
        fstype: "proc",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    // add symlinks in /dev (which is already mounted)
    InitOp::Symlink {
        linkpath: "/dev/fd",
        target: "/proc/self/fd",
    },
    InitOp::Symlink {
        linkpath: "/dev/stdin",
        target: "/proc/self/fd/0",
    },
    InitOp::Symlink {
        linkpath: "/dev/stdout",
        target: "/proc/self/fd/1",
    },
    InitOp::Symlink {
        linkpath: "/dev/stderr",
        target: "/proc/self/fd/2",
    },
    // mount tmpfs on /run and /tmp (which should already exist)
    InitOp::Mount {
        source: "tmpfs",
        target: "/run",
        fstype: "tmpfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: Some("mode=0755"),
    },
    InitOp::Mount {
        source: "tmpfs",
        target: "/tmp",
        fstype: "tmpfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    // mount shm and devpts
    InitOp::Mkdir {
        path: "/dev/shm",
        mode: 0o755,
    },
    InitOp::Mount {
        source: "shm",
        target: "/dev/shm",
        fstype: "tmpfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    InitOp::Mkdir {
        path: "/dev/pts",
        mode: 0o755,
    },
    InitOp::Mount {
        source: "devpts",
        target: "/dev/pts",
        fstype: "devpts",
        flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    // mount /sys (which should already exist)
    InitOp::Mount {
        source: "sysfs",
        target: "/sys",
        fstype: "sysfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
    InitOp::Mount {
        source: "cgroup_root",
        target: "/sys/fs/cgroup",
        fstype: "tmpfs",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: Some("mode=0755"),
    },
]);

fn init_dev() -> Result<()> {
    match mount(
        Some("dev"),
        "/dev",
        Some("devtmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        None::<&str>,
    ) {
        Ok(_) => {
            Logger::info("Mounted /dev");
            Ok(())
        }
        Err(Errno::EBUSY) => {
            Logger::info("/dev already mounted");
            Ok(())
        }
        Err(e) => {
            Logger::warn(&format!("Failed to mount /dev: {}", e));
            Ok(()) // Non-fatal
        }
    }
}

fn init_fs(ops: &[InitOp]) -> Result<()> {
    for op in ops {
        match op {
            InitOp::Mount {
                source,
                target,
                fstype,
                flags,
                data,
            } => {
                if let Err(e) = mount(Some(*source), *target, Some(*fstype), *flags, *data) {
                    Logger::warn(&format!("Failed to mount {}: {}", target, e));
                } else {
                    Logger::info(&format!("Mounted {}", target));
                }
            }
            InitOp::Mkdir { path, mode } => {
                match create_dir(*path) {
                    Ok(_) => {
                        Logger::info(&format!("Created directory {}", path));
                        let _ = fs::set_permissions(*path, std::fs::Permissions::from_mode(*mode));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                        Logger::debug(&format!("Directory {} already exists", path));
                    }
                    Err(e) => {
                        Logger::warn(&format!("Failed to create directory {}: {}", path, e));
                    }
                }
            }
            InitOp::Mknod {
                path,
                mode,
                major,
                minor,
            } => {
                let dev = makedev(*major, *minor);
                match mknod(Path::new(path), SFlag::from_bits_truncate(mode.bits()), *mode, dev) {
                    Ok(_) => Logger::info(&format!("Created device node {}", path)),
                    Err(Errno::EEXIST) => Logger::debug(&format!("Device {} already exists", path)),
                    Err(e) => Logger::warn(&format!("Failed to create device {}: {}", path, e)),
                }
            }
            InitOp::Symlink { linkpath, target } => {
                match symlinkat(*target, None, *linkpath) {
                    Ok(_) => Logger::info(&format!("Created symlink {} -> {}", linkpath, target)),
                    Err(Errno::EEXIST) => {
                        Logger::debug(&format!("Symlink {} already exists", linkpath))
                    }
                    Err(e) => Logger::warn(&format!("Failed to create symlink {}: {}", linkpath, e)),
                }
            }
        }
    }
    Ok(())
}

fn init_cgroups() -> Result<()> {
    let fpath = "/proc/cgroups";
    let file = match File::open(fpath) {
        Ok(f) => f,
        Err(e) => {
            Logger::warn(&format!("Failed to open {}: {}", fpath, e));
            return Ok(()); // Non-fatal
        }
    };

    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Skip the first line (header)
    let _ = lines.next();

    for line in lines {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 4 {
            continue;
        }

        let name = parts[0];
        let enabled = parts[3].parse::<i32>().unwrap_or(0);

        if enabled != 0 {
            let path = format!("/sys/fs/cgroup/{}", name);
            if let Err(e) = create_dir(&path) {
                Logger::warn(&format!("Failed to create cgroup dir {}: {}", path, e));
                continue;
            }

            if let Err(e) = mount(
                Some(name),
                path.as_str(),
                Some("cgroup"),
                MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
                Some(name),
            ) {
                Logger::warn(&format!("Failed to mount cgroup {}: {}", path, e));
            } else {
                Logger::info(&format!("Mounted cgroup: {}", name));
            }
        }
    }

    Ok(())
}

fn init_console() -> Result<()> {
    let console_path = "/dev/console";

    unsafe {
        let mode_r = CString::new("r").unwrap();
        let mode_w = CString::new("w").unwrap();
        let path = CString::new(console_path).unwrap();

        if libc::freopen(path.as_ptr(), mode_r.as_ptr(), libc_stdhandle::stdin()).is_null() {
            Logger::warn("Failed to reopen stdin");
        }
        if libc::freopen(path.as_ptr(), mode_w.as_ptr(), libc_stdhandle::stdout()).is_null() {
            Logger::warn("Failed to reopen stdout");
        }
        if libc::freopen(path.as_ptr(), mode_w.as_ptr(), libc_stdhandle::stderr()).is_null() {
            Logger::warn("Failed to reopen stderr");
        }
    }

    Logger::info("Console initialized");
    Ok(())
}

mod libc_stdhandle {
    extern "C" {
        #[link_name = "stdin"]
        static stdin_ptr: *mut libc::FILE;
        #[link_name = "stdout"]
        static stdout_ptr: *mut libc::FILE;
        #[link_name = "stderr"]
        static stderr_ptr: *mut libc::FILE;
    }

    pub unsafe fn stdin() -> *mut libc::FILE {
        stdin_ptr
    }

    pub unsafe fn stdout() -> *mut libc::FILE {
        stdout_ptr
    }

    pub unsafe fn stderr() -> *mut libc::FILE {
        stderr_ptr
    }
}

fn enclave_ready() -> Result<()> {
    Logger::info("Signaling enclave readiness...");

    let socket_fd = match socket(
        AddressFamily::Vsock,
        SockType::Stream,
        SockFlag::empty(),
        None,
    ) {
        Ok(fd) => fd,
        Err(e) => {
            Logger::error(&format!("Failed to create vsock socket: {}", e));
            return Ok(()); // Non-fatal
        }
    };

    let addr = VsockAddr::new(VSOCK_CID, VSOCK_PORT);

    if let Err(e) = connect(socket_fd, &addr) {
        Logger::warn(&format!("Failed to connect to vsock: {}", e));
        let _ = close(socket_fd);
        return Ok(()); // Non-fatal
    }

    let buf = [HEART_BEAT];
    if write(socket_fd, &buf).unwrap_or(0) != 1 {
        Logger::warn("Failed to write heartbeat");
        let _ = close(socket_fd);
        return Ok(()); // Non-fatal
    }

    let mut buf_read = [0u8; 1];
    if read(socket_fd, &mut buf_read).unwrap_or(0) != 1 {
        Logger::warn("Failed to read heartbeat");
        let _ = close(socket_fd);
        return Ok(()); // Non-fatal
    }

    if buf_read[0] != HEART_BEAT {
        Logger::warn("Received incorrect heartbeat");
    } else {
        Logger::info("Enclave ready signal sent successfully");
    }

    let _ = close(socket_fd);
    Ok(())
}

fn init_nsm_driver() -> Result<()> {
    use std::os::unix::io::IntoRawFd;

    let fd = match File::open(NSM_PATH) {
        Ok(f) => f.into_raw_fd(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Logger::info("NSM driver not found, skipping");
            return Ok(());
        }
        Err(e) => {
            Logger::warn(&format!("Failed to open NSM driver: {}", e));
            return Ok(()); // Non-fatal
        }
    };

    let params = CString::new("").unwrap();
    let rc = unsafe { libc::syscall(libc::SYS_finit_module, fd, params.as_ptr(), 0) };

    if rc < 0 {
        Logger::warn("Failed to insert NSM driver");
    } else {
        Logger::info("NSM driver loaded successfully");
    }

    let _ = unsafe { libc::close(fd) };

    if let Err(e) = unlink(NSM_PATH) {
        Logger::debug(&format!("Could not unlink {}: {}", NSM_PATH, e));
    }

    Ok(())
}

fn parse_service_file(path: &Path) -> Result<ServiceConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read service file: {:?}", path))?;

    let config: ServiceConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse service file: {:?}", path))?;

    Ok(config)
}

fn load_services(service_dir: &str) -> Result<HashMap<String, ServiceState>> {
    let mut services = HashMap::new();

    let entries = match read_dir(service_dir) {
        Ok(e) => e,
        Err(e) => {
            Logger::warn(&format!("Failed to read service directory {}: {}", service_dir, e));
            return Ok(services);
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("service") {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        match parse_service_file(&path) {
            Ok(config) => {
                if config.exec_start.is_empty() {
                    Logger::warn(&format!("Service {} has no ExecStart, skipping", name));
                    continue;
                }
                Logger::info(&format!("Loaded service: {}", name));
                services.insert(name.clone(), ServiceState::new(name, config));
            }
            Err(e) => {
                Logger::error(&format!("Failed to parse service {}: {}", name, e));
            }
        }
    }

    Ok(services)
}

fn launch_service(service: &mut ServiceState) -> Result<()> {
    Logger::info(&format!("Launching service: {}", service.name));

    // Parse command line
    let parts: Vec<String> = shell_words::split(&service.config.exec_start)
        .unwrap_or_else(|_| vec![service.config.exec_start.clone()]);

    if parts.is_empty() {
        Logger::error(&format!("Service {} has empty command", service.name));
        return Ok(());
    }

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            service.pid = Some(child);
            service.last_restart = Some(Instant::now());
            service.restart_count += 1;
            Logger::info(&format!(
                "Service {} started with PID {}",
                service.name, child
            ));
            Ok(())
        }
        Ok(ForkResult::Child) => {
            // Unblock signals in child
            let set = SigSet::all();
            let _ = sigprocmask(SigmaskHow::SIG_UNBLOCK, Some(&set), None);

            // Create new session
            let _ = setsid();
            let _ = setpgid(Pid::from_raw(0), Pid::from_raw(0));

            // Change working directory if specified
            if let Some(ref wd) = service.config.working_directory {
                if let Err(e) = chdir(wd.as_str()) {
                    Logger::error(&format!("Failed to chdir to {}: {}", wd, e));
                }
            }

            // Build environment
            let mut envp = service.config.environment.clone();
            envp.push(DEFAULT_PATH_ENV.to_string());

            // Convert to CStrings
            let argv_c: Vec<CString> = parts
                .iter()
                .filter_map(|s| CString::new(s.as_str()).ok())
                .collect();
            let envp_c: Vec<CString> = envp
                .iter()
                .filter_map(|s| CString::new(s.as_str()).ok())
                .collect();

            // Convert to raw pointers
            let mut argv_ptrs: Vec<*const libc::c_char> =
                argv_c.iter().map(|s| s.as_ptr()).collect();
            argv_ptrs.push(std::ptr::null());

            let mut envp_ptrs: Vec<*const libc::c_char> =
                envp_c.iter().map(|s| s.as_ptr()).collect();
            envp_ptrs.push(std::ptr::null());

            // Execute
            unsafe {
                libc::execvpe(argv_ptrs[0], argv_ptrs.as_ptr(), envp_ptrs.as_ptr());
            }

            Logger::error(&format!("Failed to exec {}", parts[0]));
            std::process::exit(1);
        }
        Err(e) => {
            Logger::error(&format!("Failed to fork for service {}: {}", service.name, e));
            Ok(())
        }
    }
}

extern "C" fn handle_sigchld(_: libc::c_int) {
    SIGCHLD_RECEIVED.store(true, Ordering::Relaxed);
}

extern "C" fn handle_sigterm(_: libc::c_int) {
    SIGTERM_RECEIVED.store(true, Ordering::Relaxed);
}

extern "C" fn handle_sigint(_: libc::c_int) {
    SIGINT_RECEIVED.store(true, Ordering::Relaxed);
}

fn setup_signal_handlers() -> Result<()> {
    let sa_chld = SigAction::new(
        SigHandler::Handler(handle_sigchld),
        SaFlags::SA_NOCLDSTOP | SaFlags::SA_RESTART,
        SigSet::empty(),
    );

    let sa_term = SigAction::new(
        SigHandler::Handler(handle_sigterm),
        SaFlags::SA_RESTART,
        SigSet::empty(),
    );

    let sa_int = SigAction::new(
        SigHandler::Handler(handle_sigint),
        SaFlags::SA_RESTART,
        SigSet::empty(),
    );

    unsafe {
        sigaction(Signal::SIGCHLD, &sa_chld)?;
        sigaction(Signal::SIGTERM, &sa_term)?;
        sigaction(Signal::SIGINT, &sa_int)?;
    }

    Logger::info("Signal handlers installed");
    Ok(())
}

fn reap_children(services: &mut HashMap<String, ServiceState>) {
    loop {
        match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::Exited(pid, status)) => {
                Logger::info(&format!("Process {} exited with status {}", pid, status));
                handle_process_exit(services, pid, status);
            }
            Ok(WaitStatus::Signaled(pid, signal, _)) => {
                let exit_code = 128 + signal as i32;
                Logger::info(&format!("Process {} killed by signal {}", pid, signal));
                handle_process_exit(services, pid, exit_code);
            }
            Ok(WaitStatus::StillAlive) => {
                break;
            }
            Err(Errno::ECHILD) => {
                // No more children
                break;
            }
            Err(e) => {
                Logger::warn(&format!("waitpid error: {}", e));
                break;
            }
            _ => {}
        }
    }
}

fn handle_process_exit(services: &mut HashMap<String, ServiceState>, pid: Pid, exit_code: i32) {
    // Find which service this PID belongs to
    for (_, service) in services.iter_mut() {
        if service.pid == Some(pid) {
            service.pid = None;
            service.exit_status = Some(exit_code);

            if service.should_restart(exit_code) {
                if service.can_restart_now() {
                    Logger::info(&format!(
                        "Service {} will be restarted (policy: {:?})",
                        service.name, service.config.restart
                    ));
                } else {
                    Logger::info(&format!(
                        "Service {} restart delayed for {} seconds",
                        service.name, service.config.restart_sec
                    ));
                }
            } else {
                Logger::info(&format!(
                    "Service {} will not be restarted (policy: {:?})",
                    service.name, service.config.restart
                ));
            }
            break;
        }
    }
}

fn restart_services(services: &mut HashMap<String, ServiceState>) {
    let mut to_restart = Vec::new();

    for (name, service) in services.iter() {
        if service.pid.is_none() {
            if let Some(exit_code) = service.exit_status {
                if service.should_restart(exit_code) && service.can_restart_now() {
                    to_restart.push(name.clone());
                }
            }
        }
    }

    for name in to_restart {
        if let Some(service) = services.get_mut(&name) {
            if let Err(e) = launch_service(service) {
                Logger::error(&format!("Failed to restart service {}: {}", name, e));
            }
        }
    }
}

fn shutdown_services(services: &mut HashMap<String, ServiceState>) {
    Logger::info("Shutting down all services...");

    // Send SIGTERM to all running services
    for (name, service) in services.iter() {
        if let Some(pid) = service.pid {
            Logger::info(&format!("Sending SIGTERM to service {} (PID {})", name, pid));
            let _ = kill(pid, Signal::SIGTERM);
        }
    }

    // Wait a bit for graceful shutdown
    thread::sleep(Duration::from_secs(5));

    // Send SIGKILL to any remaining processes
    for (name, service) in services.iter() {
        if let Some(pid) = service.pid {
            Logger::warn(&format!("Sending SIGKILL to service {} (PID {})", name, pid));
            let _ = kill(pid, Signal::SIGKILL);
        }
    }

    // Reap all children
    loop {
        match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) | Err(Errno::ECHILD) => break,
            _ => continue,
        }
    }

    Logger::info("All services stopped");
}

fn perform_pivot_root() -> Result<()> {
    Logger::info("Performing pivot root...");

    // Turn /rootfs into a mount point
    if let Err(e) = mount(
        Some("/rootfs"),
        "/rootfs",
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    ) {
        Logger::warn(&format!("Failed to bind mount /rootfs: {}", e));
        return Ok(()); // Non-fatal, might not need pivot root
    }

    if let Err(e) = chdir("/rootfs") {
        Logger::error(&format!("Failed to chdir to /rootfs: {}", e));
        return Ok(());
    }

    // Move the root filesystem
    if let Err(e) = mount(Some("."), "/", None::<&str>, MsFlags::MS_MOVE, None::<&str>) {
        Logger::warn(&format!("Failed to move mount: {}", e));
        return Ok(());
    }

    if let Err(e) = chroot(".") {
        Logger::error(&format!("Failed to chroot: {}", e));
        return Ok(());
    }

    if let Err(e) = chdir("/") {
        Logger::error(&format!("Failed to chdir to /: {}", e));
        return Ok(());
    }

    Logger::info("Pivot root completed");
    Ok(())
}

fn main() {
    Logger::init();
    Logger::info("Enclave init system starting...");

    // Setup signal handlers
    if let Err(e) = setup_signal_handlers() {
        Logger::error(&format!("Failed to setup signal handlers: {}", e));
    }

    // Initialize /dev first for early debugging
    let _ = init_dev();
    let _ = init_console();

    // Load NSM driver
    let _ = init_nsm_driver();

    // Signal enclave readiness
    let _ = enclave_ready();

    // Perform pivot root if needed
    let _ = perform_pivot_root();

    // Initialize the rest of the filesystem
    let _ = init_dev(); // Re-initialize /dev in new root
    let _ = init_fs(&OPS);
    let _ = init_cgroups();

    // Load service definitions
    let mut services = match load_services(SERVICE_DIR) {
        Ok(s) => s,
        Err(e) => {
            Logger::error(&format!("Failed to load services: {}", e));
            HashMap::new()
        }
    };

    if services.is_empty() {
        Logger::warn("No services found, init will just reap children");
    }

    // Start all services
    for (_, service) in services.iter_mut() {
        if let Err(e) = launch_service(service) {
            Logger::error(&format!("Failed to launch service {}: {}", service.name, e));
        }
    }

    // Main event loop
    Logger::info("Entering main loop");
    loop {
        // Check for shutdown signals
        if SIGTERM_RECEIVED.load(Ordering::Relaxed) || SIGINT_RECEIVED.load(Ordering::Relaxed) {
            Logger::info("Shutdown signal received");
            shutdown_services(&mut services);
            break;
        }

        // Handle SIGCHLD
        if SIGCHLD_RECEIVED.swap(false, Ordering::Relaxed) {
            reap_children(&mut services);
        }

        // Restart services that need restarting
        restart_services(&mut services);

        // Sleep briefly to avoid busy-waiting
        thread::sleep(Duration::from_millis(100));
    }

    Logger::info("Init system shutting down");

    // Reboot the system
    unsafe {
        libc::reboot(libc::RB_AUTOBOOT);
    }

    std::process::exit(0);
}

// Helper trait for setting file permissions from mode
trait FromMode {
    fn from_mode(mode: u32) -> Self;
}

impl FromMode for std::fs::Permissions {
    fn from_mode(mode: u32) -> Self {
        use std::os::unix::fs::PermissionsExt;
        <std::fs::Permissions as PermissionsExt>::from_mode(mode)
    }
}

// Simple shell word splitting helper
mod shell_words {
    pub fn split(input: &str) -> Result<Vec<String>, ()> {
        let mut words = Vec::new();
        let mut current_word = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                current_word.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => escape_next = true,
                '"' => in_quotes = !in_quotes,
                ' ' | '\t' if !in_quotes => {
                    if !current_word.is_empty() {
                        words.push(current_word.clone());
                        current_word.clear();
                    }
                }
                _ => current_word.push(ch),
            }
        }

        if !current_word.is_empty() {
            words.push(current_word);
        }

        Ok(words)
    }
}
