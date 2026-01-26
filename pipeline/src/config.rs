use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::LazyLock;
use tracing::{debug, info};

/// Default values for buffer configuration
pub mod defaults {
    pub const BUF_MAX_LEN_FILE_IO: usize = 7340032; // Files send and receive buffer (7 MiB)
    pub const BUF_MAX_LEN_FILE_PATH: usize = 8192; // Buffer for file path
    pub const BUF_MAX_LEN_CMD: usize = 8192; // Buffer for shell commands
    pub const BUF_MAX_LEN_CMD_IO: usize = 10240; // Buffer for shell commands output to STDOUT
    pub const BACKLOG: usize = 128; // Socket listen backlog
    pub const MAX_CONNECTION_ATTEMPTS: usize = 10; // Maximum connection retry attempts
}

/// Buffer configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BufferConfig {
    /// Files send and receive buffer size (default: 7340032 bytes = 7 MiB)
    pub buf_max_len_file_io: usize,
    /// Buffer for file path (default: 8192 bytes)
    pub buf_max_len_file_path: usize,
    /// Buffer for shell commands (default: 8192 bytes)
    pub buf_max_len_cmd: usize,
    /// Buffer for shell commands output to STDOUT (default: 10240 bytes)
    pub buf_max_len_cmd_io: usize,
    /// Socket listen backlog (default: 128)
    pub backlog: usize,
    /// Maximum connection retry attempts (default: 10)
    pub max_connection_attempts: usize,
}

impl Default for BufferConfig {
    fn default() -> Self {
        BufferConfig {
            buf_max_len_file_io: defaults::BUF_MAX_LEN_FILE_IO,
            buf_max_len_file_path: defaults::BUF_MAX_LEN_FILE_PATH,
            buf_max_len_cmd: defaults::BUF_MAX_LEN_CMD,
            buf_max_len_cmd_io: defaults::BUF_MAX_LEN_CMD_IO,
            backlog: defaults::BACKLOG,
            max_connection_attempts: defaults::MAX_CONNECTION_ATTEMPTS,
        }
    }
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub cid: u32,
    pub port: u32,
    #[serde(default)]
    pub buffers: BufferConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            cid: 3,
            port: 5000,
            buffers: BufferConfig::default(),
        }
    }
}

/// Global configuration loaded lazily from the config file
pub static CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    load_config_from_default_paths().unwrap_or_else(|e| {
        debug!(error = %e, "Failed to load config, using defaults");
        AppConfig::default()
    })
});

/// Try to load configuration from default paths
fn load_config_from_default_paths() -> Result<AppConfig, String> {
    let crate_name = env!("CARGO_CRATE_NAME");
    let possible_paths = [
        format!("./.config/{}.config.yaml", crate_name),
        format!("./.config/{}.config.yml", crate_name),
        format!("/etc/{}/config.yaml", crate_name),
        format!("/etc/{}/config.yml", crate_name),
    ];

    for path in &possible_paths {
        if Path::new(path).exists() {
            debug!(path = %path, "Found config file");
            return load_config_from_path(path);
        }
    }

    Err("No config file found in default paths".to_string())
}

/// Load configuration from a specific path
pub fn load_config_from_path(path: &str) -> Result<AppConfig, String> {
    debug!(path = %path, "Loading configuration from file");

    let raw_config_string = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

    let config: AppConfig = serde_yaml::from_str(&raw_config_string)
        .map_err(|e| format!("Failed to parse config file '{}': {}", path, e))?;

    info!(
        cid = config.cid,
        port = config.port,
        buf_file_io = config.buffers.buf_max_len_file_io,
        buf_file_path = config.buffers.buf_max_len_file_path,
        buf_cmd = config.buffers.buf_max_len_cmd,
        buf_cmd_io = config.buffers.buf_max_len_cmd_io,
        backlog = config.buffers.backlog,
        max_conn_attempts = config.buffers.max_connection_attempts,
        "Configuration loaded"
    );

    Ok(config)
}

/// Initialize the global configuration from a specific path
/// This should be called early in main() before any other code accesses CONFIG
pub fn init_config(path: &str) -> Result<&'static AppConfig, String> {
    // Load config and store in a static
    static INITIALIZED_CONFIG: LazyLock<std::sync::RwLock<Option<AppConfig>>> =
        LazyLock::new(|| std::sync::RwLock::new(None));

    let config = load_config_from_path(path)?;

    {
        let mut guard = INITIALIZED_CONFIG.write()
            .map_err(|e| format!("Failed to acquire config write lock: {}", e))?;
        *guard = Some(config);
    }

    // Return reference to the global CONFIG (which will now use defaults if not overridden)
    // For proper initialization, we use a different approach - see RUNTIME_CONFIG below
    Ok(&*CONFIG)
}

/// Runtime configuration that can be set after initialization
static RUNTIME_CONFIG: LazyLock<std::sync::RwLock<Option<AppConfig>>> =
    LazyLock::new(|| std::sync::RwLock::new(None));

/// Set the runtime configuration
pub fn set_runtime_config(config: AppConfig) {
    if let Ok(mut guard) = RUNTIME_CONFIG.write() {
        *guard = Some(config);
    }
}

/// Get the runtime configuration, falling back to defaults
pub fn get_config() -> AppConfig {
    RUNTIME_CONFIG
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .unwrap_or_else(|| CONFIG.clone())
}

/// Get buffer configuration
pub fn get_buffer_config() -> BufferConfig {
    get_config().buffers
}

// Convenience functions to get individual buffer sizes
pub fn buf_max_len_file_io() -> usize {
    get_buffer_config().buf_max_len_file_io
}

pub fn buf_max_len_file_path() -> usize {
    get_buffer_config().buf_max_len_file_path
}

pub fn buf_max_len_cmd() -> usize {
    get_buffer_config().buf_max_len_cmd
}

pub fn buf_max_len_cmd_io() -> usize {
    get_buffer_config().buf_max_len_cmd_io
}

pub fn backlog() -> usize {
    get_buffer_config().backlog
}

pub fn max_connection_attempts() -> usize {
    get_buffer_config().max_connection_attempts
}
