use crate::config::NitroConfig;
use crate::error::{EnclaveError, Result};
use std::process::Command;
use tracing::{info, debug};

pub struct NitroBackend;

impl NitroBackend {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn provision(&self, config: &NitroConfig) -> Result<String> {
        info!("Provisioning AWS Nitro Enclave: {}", config.enclave_name);
        
        // Build nitro-cli run command
        let mut cmd = self.build_nitro_command(config)?;
        
        debug!("Nitro CLI command: {:?}", cmd);
        
        let output = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .map_err(|e| EnclaveError::Nitro(format!("Task join error: {}", e)))?
            .map_err(|e| EnclaveError::Nitro(format!("Nitro CLI execution failed: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnclaveError::Nitro(format!(
                "Nitro CLI failed with status {}: {}",
                output.status, stderr
            )));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("Nitro Enclave {} provisioned: {}", config.enclave_name, stdout);
        
        Ok(config.enclave_name.clone())
    }
    
    fn build_nitro_command(&self, config: &NitroConfig) -> Result<Command> {
        let mut cmd = Command::new("nitro-cli");
        
        cmd.arg("run-enclave");
        
        // Enclave name
        cmd.arg("--enclave-name").arg(&config.enclave_name);
        
        // CPU configuration
        cmd.arg("--cpu-count").arg(config.cpu_count.to_string());
        
        // Memory configuration
        cmd.arg("--memory").arg(config.memory_mib.to_string());
        
        // Enclave Image File
        cmd.arg("--eif-path").arg(&config.eif_path);
        
        // VSock CID
        cmd.arg("--enclave-cid").arg(config.vsock.cid.to_string());
        
        // Debug mode
        if config.debug_mode {
            cmd.arg("--debug-mode");
        }
        
        Ok(cmd)
    }
    
    pub async fn allocate_resources(&self, config: &NitroConfig) -> Result<()> {
        info!("Allocating resources for Nitro Enclave");
        
        // Configure Nitro Enclave allocator service
        let allocator_config = format!(
            "cpu_count: {}\nmemory_mib: {}",
            config.cpu_count, config.memory_mib
        );
        
        // Write to Nitro allocator configuration
        tokio::fs::write(
            "/etc/nitro_enclaves/allocator.yaml",
            allocator_config
        )
        .await
        .map_err(|e| EnclaveError::Nitro(format!(
            "Failed to write allocator config: {}", e
        )))?;
        
        // Restart Nitro allocator service
        let output = tokio::process::Command::new("systemctl")
            .arg("restart")
            .arg("nitro-enclaves-allocator.service")
            .output()
            .await
            .map_err(|e| EnclaveError::Nitro(format!(
                "Failed to restart allocator: {}", e
            )))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnclaveError::Nitro(format!(
                "Failed to restart allocator service: {}", stderr
            )));
        }
        
        info!("Nitro Enclave resources allocated");
        Ok(())
    }
    
    pub async fn stop(&self, name: &str) -> Result<()> {
        info!("Stopping Nitro Enclave: {}", name);
        
        let output = tokio::process::Command::new("nitro-cli")
            .arg("terminate-enclave")
            .arg("--enclave-name")
            .arg(name)
            .output()
            .await
            .map_err(|e| EnclaveError::Nitro(format!("Failed to stop enclave: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnclaveError::Nitro(format!(
                "Failed to stop enclave {}: {}", name, stderr
            )));
        }
        
        info!("Nitro Enclave {} stopped", name);
        Ok(())
    }
    
    pub async fn status(&self, name: &str) -> Result<String> {
        let output = tokio::process::Command::new("nitro-cli")
            .arg("describe-enclaves")
            .output()
            .await
            .map_err(|e| EnclaveError::Nitro(format!("Failed to get status: {}", e)))?;
        
        if !output.status.success() {
            return Err(EnclaveError::Nitro("Failed to describe enclaves".to_string()));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse output to check if enclave exists
        if stdout.contains(name) {
            Ok(format!("Enclave {} is running", name))
        } else {
            Ok(format!("Enclave {} is not running", name))
        }
    }
    
    pub async fn list_enclaves(&self) -> Result<Vec<String>> {
        let output = tokio::process::Command::new("nitro-cli")
            .arg("describe-enclaves")
            .output()
            .await
            .map_err(|e| EnclaveError::Nitro(format!("Failed to list enclaves: {}", e)))?;
        
        if !output.status.success() {
            return Err(EnclaveError::Nitro("Failed to list enclaves".to_string()));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse JSON output (nitro-cli returns JSON)
        let enclaves: Vec<String> = stdout
            .lines()
            .filter(|l| l.contains("EnclaveName"))
            .map(|l| l.to_string())
            .collect();
        
        Ok(enclaves)
    }
}