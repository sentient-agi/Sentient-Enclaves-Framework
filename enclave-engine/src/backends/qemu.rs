use crate::config::{QemuConfig, TeeType, GpuVendor};
use crate::error::{EnclaveError, Result};
use std::process::Command;
use tracing::{info, debug};

pub struct QemuBackend;

impl QemuBackend {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn provision(&self, config: &QemuConfig) -> Result<String> {
        info!("Provisioning QEMU CVM: {}", config.vm.name);
        
        let mut cmd = self.build_qemu_command(config)?;
        
        debug!("QEMU command: {:?}", cmd);
        
        let output = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .map_err(|e| EnclaveError::Qemu(format!("Task join error: {}", e)))?
            .map_err(|e| EnclaveError::Qemu(format!("QEMU execution failed: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(EnclaveError::Qemu(format!(
                "QEMU failed with status {}: {}",
                output.status, stderr
            )));
        }
        
        info!("QEMU CVM {} provisioned successfully", config.vm.name);
        Ok(config.vm.name.clone())
    }
    
    fn build_qemu_command(&self, config: &QemuConfig) -> Result<Command> {
        let qemu_binary = if config.vm.qemu_binary.is_empty() {
            "/usr/bin/qemu-system-x86_64"
        } else {
            &config.vm.qemu_binary
        };
        
        let mut cmd = Command::new(qemu_binary);
        
        // Basic VM configuration
        cmd.arg("-name").arg(&config.vm.name);
        cmd.arg("-m").arg(config.vm.memory.to_string());
        cmd.arg("-smp").arg(format!("cpus={}", config.vm.cpus));
        cmd.arg("-nographic");
        cmd.arg("-nodefaults");
        cmd.arg("-serial").arg("stdio");
        
        // Disk configuration
        cmd.arg("-drive")
            .arg(format!("file={},if=virtio,format=qcow2", 
                config.vm.disk.display()));
        
        // Kernel and initrd
        cmd.arg("-kernel").arg(&config.vm.kernel);
        cmd.arg("-initrd").arg(&config.vm.initrd);
        cmd.arg("-append").arg(&config.vm.cmdline);
        
        // Network (basic virtio-net)
        cmd.arg("-netdev").arg("user,id=net0");
        cmd.arg("-device").arg("virtio-net-pci,netdev=net0");
        
        // TEE-specific configuration
        self.add_tee_config(&mut cmd, config)?;
        
        // GPU passthrough if enabled
        if let Some(gpu_config) = &config.gpu {
            if gpu_config.enable {
                self.add_gpu_config(&mut cmd, gpu_config)?;
            }
        }
        
        Ok(cmd)
    }
    
    fn add_tee_config(&self, cmd: &mut Command, config: &QemuConfig) -> Result<()> {
        match config.confidential.technology {
            TeeType::IntelTdx => {
                info!("Configuring Intel TDX");
                
                // TDX firmware
                if !config.confidential.firmware.is_empty() {
                    cmd.arg("-bios").arg(&config.confidential.firmware);
                }
                
                // TDX object
                cmd.arg("-object")
                    .arg("tdx-guest,id=tdx0,sept-ve-disable=on");
                
                // Machine configuration for TDX
                cmd.arg("-machine")
                    .arg("q35,kernel_irqchip=split,confidential-guest-support=tdx0");
                
                // CPU configuration
                cmd.arg("-cpu")
                    .arg("host,-kvm-steal-time,pmu=off");
            }
            
            TeeType::AmdSev | TeeType::AmdSevSnp => {
                let sev_type = if config.confidential.technology == TeeType::AmdSevSnp {
                    info!("Configuring AMD SEV-SNP");
                    "sev-snp-guest"
                } else {
                    info!("Configuring AMD SEV");
                    "sev-guest"
                };
                
                // SEV firmware
                if !config.confidential.firmware.is_empty() {
                    cmd.arg("-bios").arg(&config.confidential.firmware);
                }
                
                // SEV object
                let mut sev_obj = format!("{},id=sev0,cbitpos=51,reduced-phys-bits=1", sev_type);
                
                if let Some(id_key) = &config.confidential.id_key {
                    sev_obj.push_str(&format!(",kernel-hashes=on,id-key={}", 
                        id_key.display()));
                }
                
                cmd.arg("-object").arg(sev_obj);
                
                // Machine configuration for SEV
                cmd.arg("-machine")
                    .arg("q35,confidential-guest-support=sev0,memory-backend=mem0");
                
                // Memory backend
                cmd.arg("-object")
                    .arg(format!("memory-backend-memfd,id=mem0,size={}M,share=true", 
                        config.vm.memory));
                
                // CPU configuration
                cmd.arg("-cpu")
                    .arg("EPYC-v4");
            }
        }
        
        Ok(())
    }
    
    fn add_gpu_config(&self, cmd: &mut Command, gpu_config: &crate::config::GpuConfig) -> Result<()> {
        info!("Configuring GPU passthrough for {} GPUs", gpu_config.devices.len());
        
        for (idx, bdf) in gpu_config.devices.iter().enumerate() {
            match gpu_config.vendor {
                GpuVendor::Nvidia => {
                    // NVIDIA GPU passthrough with VFIO
                    cmd.arg("-device")
                        .arg(format!("vfio-pci,host={},id=gpu{}", bdf, idx));
                    
                    info!("Added NVIDIA GPU passthrough: {}", bdf);
                }
                
                GpuVendor::Amd => {
                    // AMD GPU passthrough with VFIO
                    cmd.arg("-device")
                        .arg(format!("vfio-pci,host={},id=gpu{}", bdf, idx));
                    
                    info!("Added AMD GPU passthrough: {}", bdf);
                }
            }
            
            // Add multifunction support for multi-GPU setups
            if gpu_config.devices.len() > 1 {
                cmd.arg("-device")
                    .arg(format!("vfio-pci,host={},multifunction=on", bdf));
            }
        }
        
        // Enable IOMMU for GPU passthrough
        cmd.arg("-machine").arg("iommu=on");
        
        Ok(())
    }
    
    pub async fn stop(&self, name: &str) -> Result<()> {
        info!("Stopping QEMU CVM: {}", name);
        
        // Send SIGTERM to QEMU process
        let output = tokio::process::Command::new("pkill")
            .arg("-f")
            .arg(format!("-name {}", name))
            .output()
            .await
            .map_err(|e| EnclaveError::Qemu(format!("Failed to stop QEMU: {}", e)))?;
        
        if !output.status.success() {
            return Err(EnclaveError::Qemu(format!(
                "Failed to stop QEMU CVM {}", name
            )));
        }
        
        info!("QEMU CVM {} stopped", name);
        Ok(())
    }
    
    pub async fn status(&self, name: &str) -> Result<bool> {
        let output = tokio::process::Command::new("pgrep")
            .arg("-f")
            .arg(format!("-name {}", name))
            .output()
            .await
            .map_err(|e| EnclaveError::Qemu(format!("Failed to check status: {}", e)))?;
        
        Ok(output.status.success())
    }
}