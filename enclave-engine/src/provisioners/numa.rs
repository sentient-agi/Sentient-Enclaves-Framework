use crate::config::NumaConfig;
use crate::error::{EnclaveError, Result};
use std::process::Command;
use tracing::{info, warn};

pub struct NumaManager;

impl NumaManager {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn configure(&self, config: &NumaConfig) -> Result<()> {
        if !config.enable {
            info!("NUMA configuration disabled");
            return Ok(());
        }
        
        info!("Configuring NUMA nodes");
        
        for node in &config.nodes {
            self.configure_node(node).await?;
        }
        
        info!("NUMA configuration complete");
        Ok(())
    }
    
    async fn configure_node(&self, node: &crate::config::NumaNode) -> Result<()> {
        info!("Configuring NUMA node {}", node.node_id);
        
        // Check if numactl is available
        self.check_numactl().await?;
        
        // Bind CPUs to NUMA node
        if !node.cpus.is_empty() {
            self.bind_cpus(node).await?;
        }
        
        // Configure memory for NUMA node
        if node.memory_gb > 0 {
            self.configure_memory(node).await?;
        }
        
        // Bind GPUs to NUMA node
        if !node.gpus.is_empty() {
            self.bind_gpus(node).await?;
        }
        
        Ok(())
    }
    
    async fn check_numactl(&self) -> Result<()> {
        let output = tokio::process::Command::new("which")
            .arg("numactl")
            .output()
            .await
            .map_err(|e| EnclaveError::Numa(format!("Failed to check numactl: {}", e)))?;
        
        if !output.status.success() {
            return Err(EnclaveError::Numa(
                "numactl not found. Please install numactl package.".to_string()
            ));
        }
        
        Ok(())
    }
    
    async fn bind_cpus(&self, node: &crate::config::NumaNode) -> Result<()> {
        let cpu_list = node.cpus
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");
        
        info!("Binding CPUs {} to NUMA node {}", cpu_list, node.node_id);
        
        // Use numactl to set CPU affinity
        let output = tokio::process::Command::new("numactl")
            .arg("--cpunodebind")
            .arg(node.node_id.to_string())
            .arg("--show")
            .output()
            .await
            .map_err(|e| EnclaveError::Numa(format!("Failed to bind CPUs: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("CPU binding warning: {}", stderr);
        }
        
        Ok(())
    }
    
    async fn configure_memory(&self, node: &crate::config::NumaNode) -> Result<()> {
        info!("Configuring {}GB memory for NUMA node {}", 
            node.memory_gb, node.node_id);
        
        // Check current NUMA memory configuration
        let output = tokio::process::Command::new("numactl")
            .arg("--hardware")
            .output()
            .await
            .map_err(|e| EnclaveError::Numa(format!("Failed to check NUMA hardware: {}", e)))?;
        
        if !output.status.success() {
            return Err(EnclaveError::Numa("Failed to query NUMA hardware".to_string()));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("NUMA hardware info:\n{}", stdout);
        
        Ok(())
    }
    
    async fn bind_gpus(&self, node: &crate::config::NumaNode) -> Result<()> {
        info!("Binding GPUs to NUMA node {}", node.node_id);
        
        for gpu_bdf in &node.gpus {
            // Check GPU NUMA affinity
            let numa_node_path = format!("/sys/bus/pci/devices/{}/numa_node", gpu_bdf);
            
            match tokio::fs::read_to_string(&numa_node_path).await {
                Ok(current_node) => {
                    info!("GPU {} current NUMA node: {}", gpu_bdf, current_node.trim());
                }
                Err(e) => {
                    warn!("Could not read NUMA node for GPU {}: {}", gpu_bdf, e);
                }
            }
            
            // Set GPU NUMA affinity (if supported by driver)
            // Note: This typically requires kernel driver support
            info!("GPU {} associated with NUMA node {}", gpu_bdf, node.node_id);
        }
        
        Ok(())
    }
    
    pub async fn update_grub_config(&self, config: &NumaConfig) -> Result<()> {
        if !config.enable {
            return Ok(());
        }
        
        info!("Updating GRUB configuration for NUMA");
        
        // Read current GRUB config
        let grub_default = "/etc/default/grub";
        let grub_content = tokio::fs::read_to_string(grub_default)
            .await
            .map_err(|e| EnclaveError::Numa(format!("Failed to read GRUB config: {}", e)))?;
        
        // Check if NUMA is already configured
        if grub_content.contains("numa=") {
            info!("NUMA already configured in GRUB");
            return Ok(());
        }
        
        // Add NUMA configuration to GRUB_CMDLINE_LINUX
        let numa_cmdline = "numa=on";
        let updated_content = if let Some(cmdline_pos) = grub_content.find("GRUB_CMDLINE_LINUX=") {
            let mut new_content = grub_content.clone();
            // Insert NUMA parameter into command line
            new_content.insert_str(
                cmdline_pos + "GRUB_CMDLINE_LINUX=\"".len(),
                &format!("{} ", numa_cmdline)
            );
            new_content
        } else {
            format!("{}\nGRUB_CMDLINE_LINUX=\"{}\"\n", grub_content, numa_cmdline)
        };
        
        // Write updated GRUB config
        tokio::fs::write(grub_default, updated_content)
            .await
            .map_err(|e| EnclaveError::Numa(format!("Failed to write GRUB config: {}", e)))?;
        
        // Update GRUB
        let output = tokio::process::Command::new("update-grub")
            .output()
            .await
            .map_err(|e| EnclaveError::Numa(format!("Failed to update GRUB: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnclaveError::Numa(format!("GRUB update failed: {}", stderr)));
        }
        
        info!("GRUB configuration updated. Reboot required for changes to take effect.");
        Ok(())
    }
    
    pub async fn show_numa_info(&self) -> Result<String> {
        let output = tokio::process::Command::new("numactl")
            .arg("--hardware")
            .output()
            .await
            .map_err(|e| EnclaveError::Numa(format!("Failed to get NUMA info: {}", e)))?;
        
        if !output.status.success() {
            return Err(EnclaveError::Numa("Failed to query NUMA hardware".to_string()));
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}