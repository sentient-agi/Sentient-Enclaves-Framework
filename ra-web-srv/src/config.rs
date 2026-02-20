//! Configuration management for the Remote Attestation Web Server

use crate::errors::{AppError, AppResult, ConfigError};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use tracing::{debug, error, info};
use vrf::openssl::CipherSuite;

/// Server port configuration
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Ports {
    pub http: u16,
    pub https: u16,
}

/// Cryptographic key configuration
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Keys {
    pub sk4proofs: Option<String>,
    pub sk4docs: Option<String>,
}

/// NATS message queue persistence configuration
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct NATSMQPersistency {
    pub nats_persistency_enabled: Option<i32>,
    pub nats_url: String,
    pub hash_bucket_name: String,
    pub att_docs_bucket_name: String,
    pub persistent_client_name: String,
}

/// Main configuration structure
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub nsm_fd: Option<String>,
    pub ports: Ports,
    pub keys: Keys,
    pub vrf_cipher_suite: Option<CipherSuite>,
    pub nats: Option<NATSMQPersistency>,
}

/// Thread-safe application configuration wrapper
#[derive(Default, Debug, Clone)]
pub struct AppConfig {
    pub inner: Arc<RwLock<Config>>,
}

impl AppConfig {
    /// Create a new configuration from a file path
    pub fn new_from_file(config_path: &str) -> AppResult<Self> {
        info!("Loading configuration from: {}", config_path);

        let raw_config_string = fs::read_to_string(config_path).map_err(|e| {
            error!("Failed to read config file '{}': {}", config_path, e);
            ConfigError::ReadError {
                path: config_path.to_string(),
                source: e,
            }
        })?;

        let config = serde_yaml::from_str::<Config>(raw_config_string.as_str()).map_err(|e| {
            error!("Failed to parse config file '{}': {}", config_path, e);
            ConfigError::ParseError {
                path: config_path.to_string(),
                message: e.to_string(),
            }
        })?;

        debug!("Configuration loaded successfully: {:?}", config);

        Ok(AppConfig {
            inner: Arc::new(RwLock::new(config)),
        })
    }

    /// Save the current configuration to a file
    pub fn save_to_file(&self, path: &str) -> AppResult<()> {
        info!("Saving configuration to: {}", path);

        let config = self.inner.read();
        let yaml_str = serde_yaml::to_string(&*config).map_err(|e| {
            error!("Failed to serialize config: {}", e);
            ConfigError::SerializeError(e.to_string())
        })?;

        fs::write(path, yaml_str).map_err(|e| {
            error!("Failed to write config file '{}': {}", path, e);
            ConfigError::WriteError {
                path: path.to_string(),
                source: e,
            }
        })?;

        debug!("Configuration saved successfully");
        Ok(())
    }

    /// Update the NSM file descriptor configuration
    pub fn update_nsm_fd(&self, new_nsm_fd: i32) {
        debug!("Updating NSM fd to: {}", new_nsm_fd);
        let mut config = self.inner.write();
        config.nsm_fd = Some(new_nsm_fd.to_string());
    }

    /// Update the cryptographic keys configuration
    pub fn update_keys(&self, new_keys: Keys) {
        debug!("Updating keys configuration");
        let mut config = self.inner.write();
        config.keys = Keys {
            sk4proofs: new_keys.sk4proofs,
            sk4docs: new_keys.sk4docs,
        };
    }

    /// Update the ports configuration
    pub fn update_ports(&self, new_ports: Ports) {
        debug!("Updating ports: http={}, https={}", new_ports.http, new_ports.https);
        let mut config = self.inner.write();
        config.ports = Ports {
            http: new_ports.http,
            https: new_ports.https,
        };
    }

    /// Get the NSM file descriptor
    pub fn get_nsm_fd(&self) -> AppResult<i32> {
        use aws_nitro_enclaves_nsm_api::driver::nsm_init;

        let nsm_fd = if let Some(fd) = self.inner.read().clone().nsm_fd {
            match fd.as_str() {
                "" | "nsm" | "nsm_dev" => {
                    debug!("Initializing NSM device");
                    nsm_init()
                }
                "debug" => {
                    debug!("Using debug NSM file descriptor (3)");
                    3
                }
                nsm_fd => {
                    nsm_fd.parse::<i32>().map_err(|e| {
                        error!("Failed to parse NSM fd '{}': {}", nsm_fd, e);
                        AppError::ParseError(format!("Invalid NSM fd '{}': {}", nsm_fd, e))
                    })?
                }
            }
        } else {
            debug!("NSM fd not configured, initializing NSM device");
            nsm_init()
        };

        debug!("NSM file descriptor: {}", nsm_fd);
        Ok(nsm_fd)
    }

    /// Get the cryptographic keys configuration
    pub fn get_keys(&self) -> Keys {
        self.inner.read().keys.clone()
    }

    /// Get the ports configuration
    pub fn get_ports(&self) -> Ports {
        self.inner.read().ports.clone()
    }

    /// Get the VRF cipher suite
    pub fn get_vrf_cipher_suite(&self) -> AppResult<CipherSuite> {
        let config = self.inner.read().clone();
        Ok(config.vrf_cipher_suite.ok_or_else(|| {
            error!("'vrf_cipher_suite' not present in configuration file");
            AppError::ConfigError("'vrf_cipher_suite' not present in configuration file".to_string())
        })?)
    }

    /// Get the NATS configuration
    pub fn get_nats_config(&self) -> Option<NATSMQPersistency> {
        self.inner.read().nats.clone()
    }
}
