use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const DEFAULT_CONFIG_PATH: &str = "/etc/init.yaml";
const DEFAULT_INITCTL_CONFIG_PATH: &str = "/etc/initctl.yaml";

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InitConfig {
    /// Service directory path
    pub service_dir: String,

    /// Log directory path
    pub log_dir: String,

    /// Control socket configuration
    pub control: ControlConfig,

    /// Maximum log file size in bytes
    pub max_log_size: u64,

    /// Maximum number of log files to keep
    pub max_log_files: usize,

    /// Environment variables for init system
    pub environment: HashMap<String, String>,

    /// VSOCK configuration for heartbeat
    pub vsock: VsockConfig,

    /// NSM driver path
    pub nsm_driver_path: Option<String>,

    /// Perform pivot root
    pub pivot_root: bool,

    /// Pivot root source directory
    pub pivot_root_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ControlConfig {
    /// Enable Unix socket control interface
    pub unix_socket_enabled: bool,

    /// Unix socket path
    pub unix_socket_path: String,

    /// Enable VSOCK control interface
    pub vsock_enabled: bool,

    /// VSOCK CID for control interface (usually 3 for parent, or VMADDR_CID_ANY for any)
    pub vsock_cid: u32,

    /// VSOCK port for control interface
    pub vsock_port: u32,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            unix_socket_enabled: true,
            unix_socket_path: "/run/init.sock".to_string(),
            vsock_enabled: false,
            vsock_cid: 3,
            vsock_port: 9001,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct VsockConfig {
    /// Enable VSOCK heartbeat
    pub enabled: bool,

    /// VSOCK CID for heartbeat
    pub cid: u32,

    /// VSOCK port for heartbeat
    pub port: u32,
}

impl Default for VsockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cid: 3,
            port: 9000,
        }
    }
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            service_dir: "/service".to_string(),
            log_dir: "/log".to_string(),
            control: ControlConfig::default(),
            max_log_size: 10 * 1024 * 1024, // 10 MB
            max_log_files: 5,
            environment: HashMap::new(),
            vsock: VsockConfig::default(),
            nsm_driver_path: Some("nsm.ko".to_string()),
            pivot_root: true,
            pivot_root_dir: "/rootfs".to_string(),
        }
    }
}

impl InitConfig {
    /// Load configuration from default path or environment variable
    pub fn load() -> Result<Self> {
        let config_path = std::env::var("INIT_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string());
        Self::load_from(&config_path)
    }

    /// Load configuration from specific path
    pub fn load_from(path: &str) -> Result<Self> {
        if !Path::new(path).exists() {
            eprintln!("[INFO] Config file {} not found, using defaults", path);
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        let config: InitConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

        Ok(config)
    }

    /// Apply environment variables from config
    pub fn apply_environment(&self) {
        for (key, value) in &self.environment {
            std::env::set_var(key, value);
        }
    }
}

/// Configuration for initctl client
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InitctlConfig {
    /// Control protocol to use
    pub protocol: ControlProtocol,

    /// Unix socket path
    pub unix_socket_path: String,

    /// VSOCK CID
    pub vsock_cid: u32,

    /// VSOCK port
    pub vsock_port: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ControlProtocol {
    Unix,
    Vsock,
}

impl Default for ControlProtocol {
    fn default() -> Self {
        ControlProtocol::Unix
    }
}

impl Default for InitctlConfig {
    fn default() -> Self {
        Self {
            protocol: ControlProtocol::Unix,
            unix_socket_path: "/run/init.sock".to_string(),
            vsock_cid: 3,
            vsock_port: 9001,
        }
    }
}

impl InitctlConfig {
    /// Load configuration from default path or environment variable
    pub fn load() -> Result<Self> {
        let config_path = std::env::var("INITCTL_CONFIG")
            .unwrap_or_else(|_| DEFAULT_INITCTL_CONFIG_PATH.to_string());
        Self::load_from(&config_path)
    }

    /// Load configuration from specific path
    pub fn load_from(path: &str) -> Result<Self> {
        if !Path::new(path).exists() {
            eprintln!("[INFO] Initctl config file {} not found, using defaults", path);
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read initctl config file: {}", path))?;

        let config: InitctlConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse initctl config file: {}", path))?;

        Ok(config)
    }
}
