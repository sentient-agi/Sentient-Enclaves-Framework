use serde::{Deserialize, Serialize};

pub const SOCKET_PATH: &str = "/run/init.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    ListServices,
    ServiceStatus { name: String },
    ServiceStart { name: String },
    ServiceStop { name: String },
    ServiceRestart { name: String },
    ServiceEnable { name: String },
    ServiceDisable { name: String },
    ServiceLogs { name: String, lines: usize },
    ServiceLogsClear { name: String },
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
pub struct SystemStatus {
    pub uptime_secs: u64,
    pub total_services: usize,
    pub active_services: usize,
    pub enabled_services: usize,
    pub log_dir: String,
    pub service_dir: String,
}
