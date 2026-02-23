use serde::{Deserialize, Serialize};

pub const SOCKET_PATH: &str = "/run/init.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    // Service management
    ListServices,
    ServiceStatus { name: String },
    ServiceStart { name: String },
    ServiceStop { name: String },
    ServiceRestart { name: String },
    ServiceEnable { name: String },
    ServiceDisable { name: String },
    ServiceLogs { name: String, lines: usize },
    ServiceLogsClear { name: String },

    /// Request to initialize log streaming for a service
    /// The init system will stream logs to the specified VSock address
    ServiceLogsStream {
        name: String,
        /// VSock CID to stream logs to (the host's perspective CID)
        vsock_cid: u32,
        /// VSock port to stream logs to
        vsock_port: u32,
    },

    /// Stop streaming logs for a service
    ServiceLogsStreamStop { name: String },

    // Process management
    ProcessList,
    ProcessStatus { pid: i32 },
    ProcessStart { command: String, args: Vec<String>, env: Vec<String> },
    ProcessStop { pid: i32 },
    ProcessRestart { pid: i32 },
    ProcessKill { pid: i32, signal: i32 },

    // System management
    SystemReload,
    SystemReboot,
    SystemShutdown,
    SystemStatus,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Success { message: String },
    Error { message: String },
    ServiceList { services: Vec<ServiceInfo> },
    ServiceStatus { status: ServiceStatus },
    ServiceLogs { logs: Vec<String> },
    /// Response for log streaming request
    LogsStreamStarted {
        service: String,
        vsock_cid: u32,
        vsock_port: u32,
    },
    ProcessList { processes: Vec<ProcessInfo> },
    ProcessStatus { process: ProcessInfo },
    ProcessStarted { pid: i32, message: String },
    SystemStatus { status: SystemStatus },
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub enabled: bool,
    pub active: bool,
    pub restart_policy: String,
    pub restart_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub enabled: bool,
    pub active: bool,
    pub pid: Option<i32>,
    pub restart_policy: String,
    pub restart_count: u32,
    pub restart_sec: u64,
    pub exit_status: Option<i32>,
    pub exec_start: String,
    pub working_directory: Option<String>,
    pub dependencies: ServiceDependencyInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDependencyInfo {
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub requires: Vec<String>,
    pub required_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub ppid: i32,
    pub name: String,
    pub cmdline: String,
    pub state: String,
    pub cpu_percent: f32,
    pub memory_kb: u64,
    pub start_time: u64,
    pub managed: bool,  // true if managed by init as a service
    pub service_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub uptime_secs: u64,
    pub total_services: usize,
    pub active_services: usize,
    pub enabled_services: usize,
    pub total_processes: usize,
    pub log_dir: String,
    pub service_dir: String,
}
