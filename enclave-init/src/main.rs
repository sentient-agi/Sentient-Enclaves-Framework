mod config;
mod dependencies;
mod logger;
mod process;
mod protocol;

use anyhow::{Context, Result};
use clap::Parser;
use config::InitConfig;
use dependencies::{DependencyResolver, ServiceDependencies};
use logger::{Logger, LogSubscriber, ServiceLogger};
use nix::errno::Errno;
use nix::mount::{mount, MsFlags};
use nix::sys::signal::{
    kill, sigaction, sigprocmask, SaFlags, SigAction, SigHandler, SigSet, SigmaskHow, Signal,
};
use nix::sys::socket::{
    accept, bind, connect, listen, recv, send, socket, AddressFamily, MsgFlags, SockFlag,
    SockType, SockaddrLike, UnixAddr, VsockAddr,
};
use nix::sys::stat::{makedev, mknod, Mode, SFlag};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{
    chdir, chroot, close, fork, read, setsid, setpgid, symlinkat, unlink, write, ForkResult, Pid,
};
use protocol::{Request, Response, ServiceDependencyInfo, ServiceInfo, ServiceStatus, SystemStatus};
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::{self, create_dir, read_dir, remove_file, rename, File};
use std::io::{BufRead, BufReader};
use std::os::unix::io::RawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// Constants
const DEFAULT_PATH_ENV: &str = "PATH=/sbin:/usr/sbin:/bin:/usr/bin";
const HEART_BEAT: u8 = 0xB7;

// Global flags for signal handling
static SIGCHLD_RECEIVED: AtomicBool = AtomicBool::new(false);
static SIGTERM_RECEIVED: AtomicBool = AtomicBool::new(false);
static SIGINT_RECEIVED: AtomicBool = AtomicBool::new(false);
static SIGHUP_RECEIVED: AtomicBool = AtomicBool::new(false);

// System start time for uptime calculation
static mut SYSTEM_START_TIME: Option<Instant> = None;

// CLI arguments
#[derive(Parser)]
#[command(name = "init")]
#[command(about = "Enclave init system", long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, env = "INIT_CONFIG", default_value = "/etc/init.yaml")]
    config: String,
}

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
    #[serde(default = "default_true")]
    service_enable: bool,
    #[serde(default)]
    before: Vec<String>,
    #[serde(default)]
    after: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    required_by: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum RestartPolicy {
    No,
    Always,
    OnFailure,
    OnSuccess,
}

impl RestartPolicy {
    fn as_str(&self) -> &str {
        match self {
            RestartPolicy::No => "no",
            RestartPolicy::Always => "always",
            RestartPolicy::OnFailure => "on-failure",
            RestartPolicy::OnSuccess => "on-success",
        }
    }
}

fn default_restart() -> RestartPolicy {
    RestartPolicy::No
}

fn default_restart_sec() -> u64 {
    5
}

fn default_true() -> bool {
    true
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            exec_start: String::new(),
            environment: Vec::new(),
            restart: RestartPolicy::No,
            restart_sec: 5,
            working_directory: None,
            service_enable: true,
            before: Vec::new(),
            after: Vec::new(),
            requires: Vec::new(),
            required_by: Vec::new(),
        }
    }
}

/// VSock log stream subscriber - streams logs to a VSock connection on the host.
struct VsockLogStreamer {
    socket_fd: RawFd,
    active: Arc<AtomicBool>,
    service_name: String,
    vsock_cid: u32,
    vsock_port: u32,
}

impl VsockLogStreamer {
    fn new(cid: u32, port: u32, service_name: &str) -> Result<Self> {
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

        Ok(Self {
            socket_fd,
            active: Arc::new(AtomicBool::new(true)),
            service_name: service_name.to_string(),
            vsock_cid: cid,
            vsock_port: port,
        })
    }

    fn stop(&self) {
        if self.active.swap(false, Ordering::Relaxed) {
            let _ = close(self.socket_fd);
        }
    }

    fn vsock_cid(&self) -> u32 {
        self.vsock_cid
    }

    fn vsock_port(&self) -> u32 {
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

        let data = format!("{}\n", line);
        if send(self.socket_fd, data.as_bytes(), MsgFlags::empty()).is_err() {
            self.active.store(false, Ordering::Relaxed);
        }
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
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
    logger: ServiceLogger,
    manual_stop: bool,
    enabled: bool,
}

impl ServiceState {
    fn new(
        name: String,
        config: ServiceConfig,
        log_dir: &str,
        max_log_size: u64,
        max_log_files: usize,
    ) -> Result<Self> {
        let logger = ServiceLogger::new(log_dir, &name, max_log_size, max_log_files)?;
        let enabled = config.service_enable;

        Ok(Self {
            config,
            pid: None,
            name,
            restart_count: 0,
            last_restart: None,
            exit_status: None,
            logger,
            manual_stop: false,
            enabled,
        })
    }

    fn should_restart(&self, exit_code: i32) -> bool {
        if self.manual_stop || !self.enabled {
            return false;
        }

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

    fn is_active(&self) -> bool {
        self.pid.is_some()
    }

    fn to_service_info(&self) -> ServiceInfo {
        ServiceInfo {
            name: self.name.clone(),
            enabled: self.enabled,
            active: self.is_active(),
            restart_policy: self.config.restart.as_str().to_string(),
            restart_count: self.restart_count,
        }
    }

    fn to_service_status(&self) -> ServiceStatus {
        ServiceStatus {
            name: self.name.clone(),
            enabled: self.enabled,
            active: self.is_active(),
            pid: self.pid.map(|p| p.as_raw()),
            restart_policy: self.config.restart.as_str().to_string(),
            restart_count: self.restart_count,
            restart_sec: self.config.restart_sec,
            exit_status: self.exit_status,
            exec_start: self.config.exec_start.clone(),
            working_directory: self.config.working_directory.clone(),
            dependencies: ServiceDependencyInfo {
                before: self.config.before.clone(),
                after: self.config.after.clone(),
                requires: self.config.requires.clone(),
                required_by: self.config.required_by.clone(),
            },
        }
    }

    fn get_dependencies(&self) -> ServiceDependencies {
        ServiceDependencies {
            before: self.config.before.clone(),
            after: self.config.after.clone(),
            requires: self.config.requires.clone(),
            required_by: self.config.required_by.clone(),
        }
    }
}

type ServiceMap = Arc<Mutex<HashMap<String, ServiceState>>>;
type StreamerMap = Arc<Mutex<HashMap<String, Arc<VsockLogStreamer>>>>;

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
static OPS: std::sync::LazyLock<Vec<InitOp>> = std::sync::LazyLock::new(|| vec![
    InitOp::Mount {
        source: "proc",
        target: "/proc",
        fstype: "proc",
        flags: MsFlags::MS_NODEV | MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        data: None,
    },
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
            Ok(())
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
            return Ok(());
        }
    };

    let reader = BufReader::new(file);
    let mut lines = reader.lines();

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

fn enclave_ready(config: &InitConfig) -> Result<()> {
    if !config.vsock.enabled {
        Logger::info("VSOCK heartbeat disabled in config");
        return Ok(());
    }

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
            return Ok(());
        }
    };

    let addr = VsockAddr::new(config.vsock.cid, config.vsock.port);

    if let Err(e) = connect(socket_fd, &addr) {
        Logger::warn(&format!("Failed to connect to vsock: {}", e));
        let _ = close(socket_fd);
        return Ok(());
    }

    let buf = [HEART_BEAT];
    if write(socket_fd, &buf).unwrap_or(0) != 1 {
        Logger::warn("Failed to write heartbeat");
        let _ = close(socket_fd);
        return Ok(());
    }

    let mut buf_read = [0u8; 1];
    if read(socket_fd, &mut buf_read).unwrap_or(0) != 1 {
        Logger::warn("Failed to read heartbeat");
        let _ = close(socket_fd);
        return Ok(());
    }

    if buf_read[0] != HEART_BEAT {
        Logger::warn("Received incorrect heartbeat");
    } else {
        Logger::info("Enclave ready signal sent successfully");
    }

    let _ = close(socket_fd);
    Ok(())
}

fn init_nsm_driver(config: &InitConfig) -> Result<()> {
    use std::os::unix::io::IntoRawFd;

    let nsm_path = match &config.nsm_driver_path {
        Some(path) => path,
        None => {
            Logger::info("NSM driver path not configured, skipping");
            return Ok(());
        }
    };

    let fd = match File::open(nsm_path) {
        Ok(f) => f.into_raw_fd(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Logger::info("NSM driver not found, skipping");
            return Ok(());
        }
        Err(e) => {
            Logger::warn(&format!("Failed to open NSM driver: {}", e));
            return Ok(());
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

    if let Err(e) = unlink(nsm_path.as_str()) {
        Logger::debug(&format!("Could not unlink {}: {}", nsm_path, e));
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

fn is_service_disabled(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()) == Some("disabled")
}

fn load_services(config: &InitConfig) -> Result<HashMap<String, ServiceState>> {
    let mut services = HashMap::new();

    let entries = match read_dir(&config.service_dir) {
        Ok(e) => e,
        Err(e) => {
            Logger::warn(&format!(
                "Failed to read service directory {}: {}",
                config.service_dir, e
            ));
            return Ok(services);
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        if is_service_disabled(&path) {
            Logger::info(&format!("Skipping disabled service: {:?}", path));
            continue;
        }

        if path.extension().and_then(|s| s.to_str()) != Some("service") {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        match parse_service_file(&path) {
            Ok(service_config) => {
                if service_config.exec_start.is_empty() {
                    Logger::warn(&format!("Service {} has no ExecStart, skipping", name));
                    continue;
                }

                match ServiceState::new(
                    name.clone(),
                    service_config,
                    &config.log_dir,
                    config.max_log_size,
                    config.max_log_files,
                ) {
                    Ok(state) => {
                        Logger::info(&format!(
                            "Loaded service: {} (enabled: {})",
                            name, state.enabled
                        ));
                        services.insert(name.clone(), state);
                    }
                    Err(e) => {
                        Logger::error(&format!("Failed to create logger for service {}: {}", name, e));
                    }
                }
            }
            Err(e) => {
                Logger::error(&format!("Failed to parse service {}: {}", name, e));
            }
        }
    }

    Ok(services)
}

fn compute_startup_order(services: &HashMap<String, ServiceState>) -> Vec<String> {
    let mut resolver = DependencyResolver::new();

    for (name, service) in services {
        if service.enabled {
            resolver.add_service(name.clone(), service.get_dependencies());
        }
    }

    match resolver.validate_dependencies() {
        Ok(_) => Logger::info("Service dependencies validated"),
        Err(e) => {
            Logger::error(&format!("Dependency validation failed: {}", e));
            return services.keys().cloned().collect();
        }
    }

    match resolver.compute_startup_order() {
        Ok(order) => {
            Logger::info(&format!("Computed startup order: {:?}", order));
            order
        }
        Err(e) => {
            Logger::error(&format!("Failed to compute startup order: {}", e));
            services.keys().cloned().collect()
        }
    }
}

fn launch_service(service: &mut ServiceState) -> Result<()> {
    if !service.enabled {
        Logger::warn(&format!("Service {} is disabled, not starting", service.name));
        return Ok(());
    }

    Logger::info(&format!("Launching service: {}", service.name));
    service
        .logger
        .log(format!("Starting service {}", service.name));

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
            service.manual_stop = false;
            let log_msg = format!("Service {} started with PID {}", service.name, child);
            Logger::info(&log_msg);
            service.logger.log(log_msg);
            Ok(())
        }
        Ok(ForkResult::Child) => {
            let set = SigSet::all();
            let _ = sigprocmask(SigmaskHow::SIG_UNBLOCK, Some(&set), None);

            let _ = setsid();
            let _ = setpgid(Pid::from_raw(0), Pid::from_raw(0));

            if let Some(ref wd) = service.config.working_directory {
                if let Err(e) = chdir(wd.as_str()) {
                    Logger::error(&format!("Failed to chdir to {}: {}", wd, e));
                }
            }

            let mut envp = service.config.environment.clone();
            envp.push(DEFAULT_PATH_ENV.to_string());

            let argv_c: Vec<CString> = parts
                .iter()
                .filter_map(|s| CString::new(s.as_str()).ok())
                .collect();
            let envp_c: Vec<CString> = envp
                .iter()
                .filter_map(|s| CString::new(s.as_str()).ok())
                .collect();

            let mut argv_ptrs: Vec<*const libc::c_char> =
                argv_c.iter().map(|s| s.as_ptr()).collect();
            argv_ptrs.push(std::ptr::null());

            let mut envp_ptrs: Vec<*const libc::c_char> =
                envp_c.iter().map(|s| s.as_ptr()).collect();
            envp_ptrs.push(std::ptr::null());

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

extern "C" fn handle_sighup(_: libc::c_int) {
    SIGHUP_RECEIVED.store(true, Ordering::Relaxed);
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

    let sa_hup = SigAction::new(
        SigHandler::Handler(handle_sighup),
        SaFlags::SA_RESTART,
        SigSet::empty(),
    );

    unsafe {
        sigaction(Signal::SIGCHLD, &sa_chld)?;
        sigaction(Signal::SIGTERM, &sa_term)?;
        sigaction(Signal::SIGINT, &sa_int)?;
        sigaction(Signal::SIGHUP, &sa_hup)?;
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
    for (_, service) in services.iter_mut() {
        if service.pid == Some(pid) {
            service.pid = None;
            service.exit_status = Some(exit_code);

            let log_msg = format!("Service {} exited with code {}", service.name, exit_code);
            service.logger.log(log_msg.clone());
            Logger::info(&log_msg);

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
        if service.pid.is_none() && service.enabled {
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

    for (name, service) in services.iter() {
        if let Some(pid) = service.pid {
            Logger::info(&format!("Sending SIGTERM to service {} (PID {})", name, pid));
            let _ = kill(pid, Signal::SIGTERM);
        }
    }

    thread::sleep(Duration::from_secs(5));

    for (name, service) in services.iter() {
        if let Some(pid) = service.pid {
            Logger::warn(&format!("Sending SIGKILL to service {} (PID {})", name, pid));
            let _ = kill(pid, Signal::SIGKILL);
        }
    }

    loop {
        match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) | Err(Errno::ECHILD) => break,
            _ => continue,
        }
    }

    Logger::info("All services stopped");
}

fn perform_pivot_root(config: &InitConfig) -> Result<()> {
    if !config.pivot_root {
        Logger::info("Pivot root disabled in config");
        return Ok(());
    }

    Logger::info("Performing pivot root...");

    let pivot_dir = &config.pivot_root_dir;

    if let Err(e) = mount(
        Some(pivot_dir.as_str()),
        pivot_dir.as_str(),
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    ) {
        Logger::warn(&format!("Failed to bind mount {}: {}", pivot_dir, e));
        return Ok(());
    }

    if let Err(e) = chdir(pivot_dir.as_str()) {
        Logger::error(&format!("Failed to chdir to {}: {}", pivot_dir, e));
        return Ok(());
    }

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

fn get_system_status(config: &InitConfig, services: &HashMap<String, ServiceState>) -> SystemStatus {
    let uptime = unsafe {
        SYSTEM_START_TIME
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0)
    };

    let active_services = services.values().filter(|s| s.is_active()).count();
    let enabled_services = services.values().filter(|s| s.enabled).count();

    let total_processes = if let Ok(entries) = fs::read_dir("/proc") {
        entries.filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().parse::<i32>().is_ok())
            .count()
    } else {
        0
    };

    SystemStatus {
        uptime_secs: uptime,
        total_services: services.len(),
        active_services,
        enabled_services,
        total_processes,
        log_dir: config.log_dir.clone(),
        service_dir: config.service_dir.clone(),
    }
}

fn get_service_pids(services: &HashMap<String, ServiceState>) -> HashMap<String, i32> {
    services
        .iter()
        .filter_map(|(name, service)| {
            service.pid.map(|pid| (name.clone(), pid.as_raw()))
        })
        .collect()
}

fn enable_service(config: &InitConfig, name: &str) -> Result<(), String> {
    let disabled_path = PathBuf::from(&config.service_dir)
        .join(format!("{}.service.disabled", name));
    let enabled_path = PathBuf::from(&config.service_dir)
        .join(format!("{}.service", name));

    if disabled_path.exists() {
        rename(&disabled_path, &enabled_path)
            .map_err(|e| format!("Failed to rename service file: {}", e))?;
        Ok(())
    } else if enabled_path.exists() {
        Ok(())
    } else {
        Err(format!("Service file not found"))
    }
}

fn disable_service(config: &InitConfig, name: &str) -> Result<(), String> {
    let enabled_path = PathBuf::from(&config.service_dir)
        .join(format!("{}.service", name));
    let disabled_path = PathBuf::from(&config.service_dir)
        .join(format!("{}.service.disabled", name));

    if enabled_path.exists() {
        rename(&enabled_path, &disabled_path)
            .map_err(|e| format!("Failed to rename service file: {}", e))?;
        Ok(())
    } else if disabled_path.exists() {
        Ok(())
    } else {
        Err(format!("Service file not found"))
    }
}

fn reload_services(config: &InitConfig) -> Result<HashMap<String, ServiceState>, String> {
    Logger::info("Reloading service configurations...");
    load_services(config).map_err(|e| e.to_string())
}

fn handle_client_request(
    request: Request,
    services: &ServiceMap,
    config: &InitConfig,
    streamers: &StreamerMap,
) -> Response {
    match request {
        Request::Ping => Response::Pong,

        Request::ListServices => {
            let services = services.lock().unwrap();
            let service_list: Vec<ServiceInfo> = services
                .values()
                .map(|s| s.to_service_info())
                .collect();
            Response::ServiceList {
                services: service_list,
            }
        }

        Request::ServiceStatus { name } => {
            let services = services.lock().unwrap();
            match services.get(&name) {
                Some(service) => Response::ServiceStatus {
                    status: service.to_service_status(),
                },
                None => Response::Error {
                    message: format!("Service '{}' not found", name),
                },
            }
        }

        Request::ServiceStart { name } => {
            let mut services = services.lock().unwrap();
            match services.get_mut(&name) {
                Some(service) => {
                    if !service.enabled {
                        Response::Error {
                            message: format!("Service '{}' is disabled", name),
                        }
                    } else if service.is_active() {
                        Response::Error {
                            message: format!("Service '{}' is already running", name),
                        }
                    } else {
                        service.manual_stop = false;
                        match launch_service(service) {
                            Ok(_) => Response::Success {
                                message: format!("Service '{}' started", name),
                            },
                            Err(e) => Response::Error {
                                message: format!("Failed to start service '{}': {}", name, e),
                            },
                        }
                    }
                }
                None => Response::Error {
                    message: format!("Service '{}' not found", name),
                },
            }
        }

        Request::ServiceStop { name } => {
            let mut services = services.lock().unwrap();
            match services.get_mut(&name) {
                Some(service) => {
                    if let Some(pid) = service.pid {
                        service.manual_stop = true;
                        if let Err(e) = kill(pid, Signal::SIGTERM) {
                            Response::Error {
                                message: format!("Failed to stop service '{}': {}", name, e),
                            }
                        } else {
                            service.logger.log(format!("Service {} stopped manually", name));
                            Response::Success {
                                message: format!("Service '{}' stop signal sent", name),
                            }
                        }
                    } else {
                        Response::Error {
                            message: format!("Service '{}' is not running", name),
                        }
                    }
                }
                None => Response::Error {
                    message: format!("Service '{}' not found", name),
                },
            }
        }

        Request::ServiceRestart { name } => {
            let mut services = services.lock().unwrap();
            match services.get_mut(&name) {
                Some(service) => {
                    if !service.enabled {
                        Response::Error {
                            message: format!("Service '{}' is disabled", name),
                        }
                    } else {
                        if let Some(pid) = service.pid {
                            let _ = kill(pid, Signal::SIGTERM);
                            thread::sleep(Duration::from_millis(500));
                        }
                        service.manual_stop = false;
                        match launch_service(service) {
                            Ok(_) => Response::Success {
                                message: format!("Service '{}' restarted", name),
                            },
                            Err(e) => Response::Error {
                                message: format!("Failed to restart service '{}': {}", name, e),
                            },
                        }
                    }
                }
                None => Response::Error {
                    message: format!("Service '{}' not found", name),
                },
            }
        }

        Request::ServiceEnable { name } => {
            match enable_service(config, &name) {
                Ok(_) => {
                    SIGHUP_RECEIVED.store(true, Ordering::Relaxed);
                    Response::Success {
                        message: format!("Service '{}' enabled", name),
                    }
                }
                Err(e) => Response::Error {
                    message: format!("Failed to enable service '{}': {}", name, e),
                },
            }
        }

        Request::ServiceDisable { name } => {
            let mut services = services.lock().unwrap();
            if let Some(service) = services.get_mut(&name) {
                if let Some(pid) = service.pid {
                    let _ = kill(pid, Signal::SIGTERM);
                }
            }
            drop(services);

            match disable_service(config, &name) {
                Ok(_) => {
                    SIGHUP_RECEIVED.store(true, Ordering::Relaxed);
                    Response::Success {
                        message: format!("Service '{}' disabled", name),
                    }
                }
                Err(e) => Response::Error {
                    message: format!("Failed to disable service '{}': {}", name, e),
                },
            }
        }

        Request::ServiceLogs { name, lines } => {
            let services = services.lock().unwrap();
            match services.get(&name) {
                Some(service) => Response::ServiceLogs {
                    logs: service.logger.get_logs(lines),
                },
                None => Response::Error {
                    message: format!("Service '{}' not found", name),
                },
            }
        }

        Request::ServiceLogsClear { name } => {
            let services = services.lock().unwrap();
            match services.get(&name) {
                Some(service) => match service.logger.clear() {
                    Ok(_) => Response::Success {
                        message: format!("Logs cleared for service '{}'", name),
                    },
                    Err(e) => Response::Error {
                        message: format!("Failed to clear logs: {}", e),
                    },
                },
                None => Response::Error {
                    message: format!("Service '{}' not found", name),
                },
            }
        }

        Request::ServiceLogsStream { name, vsock_cid, vsock_port } => {
            let services = services.lock().unwrap();
            match services.get(&name) {
                Some(service) => {
                    {
                        let mut streamers_guard = streamers.lock().unwrap();
                        if let Some(existing) = streamers_guard.get(&name) {
                            if existing.is_active() {
                                return Response::Error {
                                    message: format!(
                                        "Log streaming already active for service '{}' on CID:{} PORT:{}",
                                        name,
                                        existing.vsock_cid(),
                                        existing.vsock_port()
                                    ),
                                };
                            }
                            streamers_guard.remove(&name);
                        }
                    }

                    match VsockLogStreamer::new(vsock_cid, vsock_port, &name) {
                        Ok(streamer) => {
                            let streamer = Arc::new(streamer);
                            service.logger.subscribe(streamer.clone());

                            {
                                let mut streamers_guard = streamers.lock().unwrap();
                                streamers_guard.insert(name.clone(), streamer);
                            }

                            Logger::info(&format!(
                                "Log streaming started for service '{}' to CID:{} PORT:{}",
                                name, vsock_cid, vsock_port
                            ));

                            Response::LogsStreamStarted {
                                service: name,
                                vsock_cid,
                                vsock_port,
                            }
                        }
                        Err(e) => {
                            Logger::error(&format!(
                                "Failed to start log streaming for '{}': {}",
                                name, e
                            ));
                            Response::Error {
                                message: format!("Failed to start log streaming: {}", e),
                            }
                        }
                    }
                }
                None => Response::Error {
                    message: format!("Service '{}' not found", name),
                },
            }
        }

        Request::ServiceLogsStreamStop { name } => {
            let mut streamers_guard = streamers.lock().unwrap();
            if let Some(streamer) = streamers_guard.remove(&name) {
                streamer.stop();
                Logger::info(&format!("Log streaming stopped for service '{}'", name));
                Response::Success {
                    message: format!("Log streaming stopped for service '{}'", name),
                }
            } else {
                Response::Error {
                    message: format!("No active log stream for service '{}'", name),
                }
            }
        }

        Request::ProcessList => {
            let services = services.lock().unwrap();
            let service_pids = get_service_pids(&services);
            let processes = process::list_processes(&service_pids);
            Response::ProcessList { processes }
        }

        Request::ProcessStatus { pid } => {
            let services = services.lock().unwrap();
            let service_pids = get_service_pids(&services);
            let uptime_secs = unsafe {
                SYSTEM_START_TIME
                    .map(|start| start.elapsed().as_secs())
                    .unwrap_or(0)
            };

            match process::get_process_info(pid, uptime_secs, &service_pids) {
                Ok(process) => Response::ProcessStatus { process },
                Err(e) => Response::Error {
                    message: format!("Failed to get process status: {}", e),
                },
            }
        }

        Request::ProcessStart { command, args, env } => {
            match process::start_process(&command, &args, &env) {
                Ok(pid) => Response::ProcessStarted {
                    pid,
                    message: format!("Process started with PID {}", pid),
                },
                Err(e) => Response::Error {
                    message: format!("Failed to start process: {}", e),
                },
            }
        }

        Request::ProcessStop { pid } => {
            match process::signal_process(pid, Signal::SIGTERM) {
                Ok(_) => Response::Success {
                    message: format!("SIGTERM sent to process {}", pid),
                },
                Err(e) => Response::Error {
                    message: format!("Failed to stop process: {}", e),
                },
            }
        }

        Request::ProcessRestart { pid } => {
            let service_pids = {
                let services_guard = services.lock().unwrap();
                get_service_pids(&services_guard)
            };

            if let Some((service_name, _)) = service_pids.iter().find(|(_, &p)| p == pid) {
                return handle_client_request(
                    Request::ServiceRestart { name: service_name.clone() },
                    services,
                    config,
                    streamers,
                );
            }

            Response::Error {
                message: format!("Process {} is not managed by init, cannot restart", pid),
            }
        }

        Request::ProcessKill { pid, signal } => {
            let sig = match signal {
                1 => Signal::SIGHUP,
                2 => Signal::SIGINT,
                9 => Signal::SIGKILL,
                15 => Signal::SIGTERM,
                _ => {
                    return Response::Error {
                        message: format!("Unsupported signal: {}", signal),
                    };
                }
            };

            match process::signal_process(pid, sig) {
                Ok(_) => Response::Success {
                    message: format!("Signal {} sent to process {}", signal, pid),
                },
                Err(e) => Response::Error {
                    message: format!("Failed to send signal: {}", e),
                },
            }
        }

        Request::SystemStatus => {
            let services = services.lock().unwrap();
            Response::SystemStatus {
                status: get_system_status(config, &services),
            }
        }

        Request::SystemReload => {
            Logger::info("Reload requested via control socket");
            SIGHUP_RECEIVED.store(true, Ordering::Relaxed);
            Response::Success {
                message: "System reload initiated".to_string(),
            }
        }

        Request::SystemReboot => {
            Logger::info("Reboot requested via control socket");
            SIGTERM_RECEIVED.store(true, Ordering::Relaxed);
            Response::Success {
                message: "System reboot initiated".to_string(),
            }
        }

        Request::SystemShutdown => {
            Logger::info("Shutdown requested via control socket");
            SIGTERM_RECEIVED.store(true, Ordering::Relaxed);
            Response::Success {
                message: "System shutdown initiated".to_string(),
            }
        }
    }
}

fn handle_connection(fd: RawFd, services: &ServiceMap, config: &InitConfig, streamers: &StreamerMap) {
    let mut buffer = vec![0u8; 8192];

    match recv(fd, &mut buffer, MsgFlags::empty()) {
        Ok(n) if n > 0 => {
            buffer.truncate(n);
            match serde_json::from_slice::<Request>(&buffer) {
                Ok(request) => {
                    let response = handle_client_request(request, services, config, streamers);
                    let response_data = match serde_json::to_vec(&response) {
                        Ok(data) => data,
                        Err(e) => {
                            Logger::error(&format!("Failed to serialize response: {}", e));
                            let _ = close(fd);
                            return;
                        }
                    };

                    let _ = send(fd, &response_data, MsgFlags::empty());
                }
                Err(e) => {
                    Logger::warn(&format!("Failed to parse request: {}", e));
                    let error_response = Response::Error {
                        message: format!("Invalid request: {}", e),
                    };
                    if let Ok(data) = serde_json::to_vec(&error_response) {
                        let _ = send(fd, &data, MsgFlags::empty());
                    }
                }
            }
        }
        Ok(_) => {
            Logger::debug("Empty request received");
        }
        Err(e) => {
            Logger::warn(&format!("Failed to receive data: {}", e));
        }
    }

    let _ = close(fd);
}

fn unix_socket_thread(services: ServiceMap, config: InitConfig, streamers: StreamerMap) {
    let socket_path = &config.control.unix_socket_path;
    let _ = remove_file(socket_path);

    let socket_fd = match socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::empty(),
        None,
    ) {
        Ok(fd) => fd,
        Err(e) => {
            Logger::error(&format!("Failed to create Unix socket: {}", e));
            return;
        }
    };

    let addr = match UnixAddr::new(socket_path.as_str()) {
        Ok(addr) => addr,
        Err(e) => {
            Logger::error(&format!("Failed to create Unix socket address: {}", e));
            return;
        }
    };

    if let Err(e) = bind(socket_fd, &addr) {
        Logger::error(&format!("Failed to bind Unix socket: {}", e));
        return;
    }

    if let Err(e) = listen(socket_fd, 5) {
        Logger::error(&format!("Failed to listen on Unix socket: {}", e));
        return;
    }

    Logger::info(&format!("Unix control socket listening on {}", socket_path));

    loop {
        match accept(socket_fd) {
            Ok(client_fd) => {
                let services = services.clone();
                let config = config.clone();
                let streamers = streamers.clone();
                thread::spawn(move || {
                    handle_connection(client_fd, &services, &config, &streamers);
                });
            }
            Err(e) => {
                Logger::warn(&format!("Failed to accept Unix connection: {}", e));
            }
        }
    }
}

fn vsock_socket_thread(services: ServiceMap, config: InitConfig, streamers: StreamerMap) {
    let socket_fd = match socket(
        AddressFamily::Vsock,
        SockType::Stream,
        SockFlag::empty(),
        None,
    ) {
        Ok(fd) => fd,
        Err(e) => {
            Logger::error(&format!("Failed to create VSOCK socket: {}", e));
            return;
        }
    };

    let listen_addr = VsockAddr::new(config.control.vsock_cid, config.control.vsock_port);

    if let Err(e) = bind(socket_fd, &listen_addr) {
        Logger::error(&format!("Failed to bind VSOCK socket: {}", e));
        return;
    }

    if let Err(e) = listen(socket_fd, 5) {
        Logger::error(&format!("Failed to listen on VSOCK socket: {}", e));
        return;
    }

    Logger::info(&format!(
        "VSOCK control socket listening on CID:{} PORT:{}",
        config.control.vsock_cid, config.control.vsock_port
    ));

    loop {
        match accept(socket_fd) {
            Ok(client_fd) => {
                let services = services.clone();
                let config = config.clone();
                let streamers = streamers.clone();
                thread::spawn(move || {
                    handle_connection(client_fd, &services, &config, &streamers);
                });
            }
            Err(e) => {
                Logger::warn(&format!("Failed to accept VSOCK connection: {}", e));
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    Logger::init();
    Logger::info("Enclave init system starting...");
    Logger::info(&format!("Loading configuration from: {}", args.config));

    unsafe {
        SYSTEM_START_TIME = Some(Instant::now());
    }

    let config = match InitConfig::load_from(&args.config) {
        Ok(c) => {
            Logger::info("Configuration loaded successfully");
            c
        }
        Err(e) => {
            Logger::error(&format!("Failed to load config: {}, using defaults", e));
            InitConfig::default()
        }
    };

    config.apply_environment();
    Logger::info(&format!("Service directory: {}", config.service_dir));
    Logger::info(&format!("Log directory: {}", config.log_dir));

    if let Err(e) = fs::create_dir_all(&config.log_dir) {
        Logger::warn(&format!("Failed to create log directory: {}", e));
    }

    if let Err(e) = setup_signal_handlers() {
        Logger::error(&format!("Failed to setup signal handlers: {}", e));
    }

    let _ = init_dev();
    let _ = init_console();
    let _ = init_nsm_driver(&config);
    let _ = enclave_ready(&config);
    let _ = perform_pivot_root(&config);
    let _ = init_dev();
    let _ = init_fs(&OPS);
    let _ = init_cgroups();

    if let Err(e) = fs::create_dir_all(&config.log_dir) {
        Logger::warn(&format!("Failed to create log directory after pivot root: {}", e));
    }

    let services_map: ServiceMap = match load_services(&config) {
        Ok(s) => Arc::new(Mutex::new(s)),
        Err(e) => {
            Logger::error(&format!("Failed to load services: {}", e));
            Arc::new(Mutex::new(HashMap::new()))
        }
    };

    let streamers: StreamerMap = Arc::new(Mutex::new(HashMap::new()));

    {
        let services = services_map.lock().unwrap();
        if services.is_empty() {
            Logger::warn("No services found, init will just reap children");
        } else {
            let startup_order = compute_startup_order(&services);
            drop(services);

            for service_name in startup_order {
                let mut services = services_map.lock().unwrap();
                if let Some(service) = services.get_mut(&service_name) {
                    if service.enabled {
                        if let Err(e) = launch_service(service) {
                            Logger::error(&format!("Failed to launch service {}: {}", service_name, e));
                        }
                        drop(services);
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
    }

    if config.control.unix_socket_enabled {
        let services_for_unix = services_map.clone();
        let config_for_unix = config.clone();
        let streamers_for_unix = streamers.clone();
        thread::spawn(move || {
            unix_socket_thread(services_for_unix, config_for_unix, streamers_for_unix);
        });
    }

    if config.control.vsock_enabled {
        let services_for_vsock = services_map.clone();
        let config_for_vsock = config.clone();
        let streamers_for_vsock = streamers.clone();
        thread::spawn(move || {
            vsock_socket_thread(services_for_vsock, config_for_vsock, streamers_for_vsock);
        });
    }

    Logger::info("Entering main loop");
    loop {
        if SIGTERM_RECEIVED.load(Ordering::Relaxed) || SIGINT_RECEIVED.load(Ordering::Relaxed) {
            Logger::info("Shutdown signal received");

            {
                let mut streamers_guard = streamers.lock().unwrap();
                for (name, streamer) in streamers_guard.drain() {
                    Logger::info(&format!("Stopping log streamer for service '{}'", name));
                    streamer.stop();
                }
            }

            let mut services = services_map.lock().unwrap();
            shutdown_services(&mut services);
            break;
        }

        if SIGHUP_RECEIVED.swap(false, Ordering::Relaxed) {
            Logger::info("Reload signal received");
            match reload_services(&config) {
                Ok(new_services) => {
                    let mut services = services_map.lock().unwrap();

                    for (name, service) in services.iter_mut() {
                        if !new_services.contains_key(name) || !new_services[name].enabled {
                            if let Some(pid) = service.pid {
                                Logger::info(&format!("Stopping removed/disabled service: {}", name));
                                let _ = kill(pid, Signal::SIGTERM);
                            }
                        }
                    }

                    *services = new_services;
                    Logger::info("Services reloaded successfully");

                    let startup_order = compute_startup_order(&services);
                    for service_name in startup_order {
                        if let Some(service) = services.get_mut(&service_name) {
                            if service.enabled && !service.is_active() {
                                if let Err(e) = launch_service(service) {
                                    Logger::error(&format!("Failed to start service {}: {}", service_name, e));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    Logger::error(&format!("Failed to reload services: {}", e));
                }
            }
        }

        if SIGCHLD_RECEIVED.swap(false, Ordering::Relaxed) {
            let mut services = services_map.lock().unwrap();
            reap_children(&mut services);
        }

        {
            let mut services = services_map.lock().unwrap();
            restart_services(&mut services);
        }

        {
            let mut streamers_guard = streamers.lock().unwrap();
            streamers_guard.retain(|name, streamer| {
                let active = streamer.is_active();
                if !active {
                    Logger::debug(&format!("Removing inactive log streamer for '{}'", name));
                }
                active
            });
        }

        thread::sleep(Duration::from_millis(100));
    }

    Logger::info("Init system shutting down");

    unsafe {
        libc::reboot(libc::RB_AUTOBOOT);
    }

    std::process::exit(0);
}

trait FromMode {
    fn from_mode(mode: u32) -> Self;
}

impl FromMode for std::fs::Permissions {
    fn from_mode(mode: u32) -> Self {
        use std::os::unix::fs::PermissionsExt;
        <std::fs::Permissions as PermissionsExt>::from_mode(mode)
    }
}

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
