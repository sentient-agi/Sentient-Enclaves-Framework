use crate::config::HugepagesConfig;
use crate::error::{EnclaveError, Result};
use std::path::PathBuf;
use tracing::{info, warn};

pub struct HugepagesManager;

impl HugepagesManager {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn configure(&self, config: &HugepagesConfig) -> Result<()> {
        if !config.enable {
            info!("Hugepages configuration disabled");
            return Ok(());
        }
        
        info!("Configuring hugepages: {} pages of {} KB", 
            config.num_pages, config.page_size_kb);
        
        // Check if hugepages are supported
        self.check_hugepage_support().await?;
        
        // Configure hugepage pool
        self.allocate_hugepages(config).await?;
        
        // Mount hugetlbfs if mount point specified
        if let Some(mount_point) = &config.mount_point {
            self.mount_hugetlbfs(mount_point, config.page_size_kb).await?;
        }
        
        info!("Hugepages configuration complete");
        Ok(())
    }
    
    async fn check_hugepage_support(&self) -> Result<()> {
        let meminfo = tokio::fs::read_to_string("/proc/meminfo")
            .await
            .map_err(|e| EnclaveError::Hugepages(format!("Failed to read meminfo: {}", e)))?;
        
        if !meminfo.contains("Hugepagesize") {
            return Err(EnclaveError::Hugepages(
                "Hugepages not supported by kernel".to_string()
            ));
        }
        
        info!("Hugepages support detected");
        Ok(())
    }
    
    async fn allocate_hugepages(&self, config: &HugepagesConfig) -> Result<()> {
        // Determine the sysfs path based on page size
        let sysfs_path = match config.page_size_kb {
            2048 => "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages",
            1048576 => "/sys/kernel/mm/hugepages/hugepages-1048576kB/nr_hugepages",
            _ => return Err(EnclaveError::Hugepages(
                format!("Unsupported hugepage size: {} KB", config.page_size_kb)
            )),
        };
        
        // Write number of pages to sysfs
        tokio::fs::write(sysfs_path, config.num_pages.to_string())
            .await
            .map_err(|e| EnclaveError::Hugepages(format!(
                "Failed to allocate hugepages: {}. Try running with sudo.", e
            )))?;
        
        // Verify allocation
        let allocated = tokio::fs::read_to_string(sysfs_path)
            .await
            .map_err(|e| EnclaveError::Hugepages(format!("Failed to verify allocation: {}", e)))?;
        
        let allocated_pages: u64 = allocated.trim().parse()
            .map_err(|e| EnclaveError::Hugepages(format!("Failed to parse allocation: {}", e)))?;
        
        if allocated_pages < config.num_pages {
            warn!("Only {} of {} hugepages allocated", allocated_pages, config.num_pages);
        } else {
            info!("Successfully allocated {} hugepages", allocated_pages);
        }
        
        Ok(())
    }
    
    async fn mount_hugetlbfs(&self, mount_point: &PathBuf, page_size_kb: u64) -> Result<()> {
        info!("Mounting hugetlbfs at {}", mount_point.display());
        
        // Create mount point if it doesn't exist
        if !mount_point.exists() {
            tokio::fs::create_dir_all(mount_point)
                .await
                .map_err(|e| EnclaveError::Hugepages(format!(
                    "Failed to create mount point: {}", e
                )))?;
        }
        
        // Check if already mounted
        let mounts = tokio::fs::read_to_string("/proc/mounts")
            .await
            .map_err(|e| EnclaveError::Hugepages(format!("Failed to read mounts: {}", e)))?;
        
        if mounts.contains(&mount_point.to_string_lossy().to_string()) {
            info!("Hugetlbfs already mounted at {}", mount_point.display());
            return Ok(());
        }
        
        // Mount hugetlbfs
        let pagesize_arg = format!("pagesize={}K", page_size_kb);
        let output = tokio::process::Command::new("mount")
            .arg("-t")
            .arg("hugetlbfs")
            .arg("-o")
            .arg(&pagesize_arg)
            .arg("none")
            .arg(mount_point)
            .output()
            .await
            .map_err(|e| EnclaveError::Hugepages(format!("Failed to mount hugetlbfs: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnclaveError::Hugepages(format!(
                "Mount failed: {}", stderr
            )));
        }
        
        info!("Hugetlbfs mounted successfully");
        Ok(())
    }
    
    pub async fn configure_kernel_params(&self, config: &HugepagesConfig) -> Result<()> {
        if !config.enable {
            return Ok(());
        }
        
        info!("Configuring kernel parameters for hugepages");
        
        let sysctl_conf = "/etc/sysctl.conf";
        let mut sysctl_content = tokio::fs::read_to_string(sysctl_conf)
            .await
            .unwrap_or_default();
        
        // Add hugepages kernel parameters
        let hugepage_param = format!(
            "\n# Hugepages configuration\nvm.nr_hugepages = {}\n",
            config.num_pages
        );
        
        if !sysctl_content.contains("vm.nr_hugepages") {
            sysctl_content.push_str(&hugepage_param);
            
            tokio::fs::write(sysctl_conf, sysctl_content)
                .await
                .map_err(|e| EnclaveError::Hugepages(format!(
                    "Failed to write sysctl.conf: {}", e
                )))?;
            
            // Apply sysctl changes
            let output = tokio::process::Command::new("sysctl")
                .arg("-p")
                .output()
                .await
                .map_err(|e| EnclaveError::Hugepages(format!(
                    "Failed to apply sysctl: {}", e
                )))?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("sysctl apply warning: {}", stderr);
            }
            
            info!("Kernel parameters configured");
        }
        
        Ok(())
    }
    
    pub async fn show_hugepage_info(&self) -> Result<String> {
        let meminfo = tokio::fs::read_to_string("/proc/meminfo")
            .await
            .map_err(|e| EnclaveError::Hugepages(format!("Failed to read meminfo: {}", e)))?;
        
        let hugepage_lines: Vec<&str> = meminfo
            .lines()
            .filter(|l| l.contains("Huge"))
            .collect();
        
        Ok(hugepage_lines.join("\n"))
    }
    
    pub async fn free_hugepages(&self) -> Result<()> {
        info!("Freeing hugepages");
        
        // Reset 2MB hugepages
        let path_2mb = "/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages";
        if tokio::fs::metadata(path_2mb).await.is_ok() {
            tokio::fs::write(path_2mb, "0")
                .await
                .map_err(|e| EnclaveError::Hugepages(format!("Failed to free 2MB pages: {}", e)))?;
        }
        
        // Reset 1GB hugepages
        let path_1gb = "/sys/kernel/mm/hugepages/hugepages-1048576kB/nr_hugepages";
        if tokio::fs::metadata(path_1gb).await.is_ok() {
            tokio::fs::write(path_1gb, "0")
                .await
                .map_err(|e| EnclaveError::Hugepages(format!("Failed to free 1GB pages: {}", e)))?;
        }
        
        info!("Hugepages freed");
        Ok(())
    }
}