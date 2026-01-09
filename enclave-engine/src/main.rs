mod config;
mod error;
mod service;
mod api;
mod backends {
    pub mod qemu;
    pub mod nitro;
}
mod provisioners {
    pub mod numa;
    pub mod hugepages;
}

use crate::service::EnclaveService;
use crate::api::create_router;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "enclave_engine=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    info!("Starting Enclave Engine Service");
    
    // Create service instance
    let service = EnclaveService::new();
    
    // Create API router
    let app = create_router(service);
    
    // Bind to address
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Listening on {}", addr);
    
    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_config_parsing() {
        let yaml = r#"
general:
  name: test-enclave
  backend: qemu

qemu:
  vm:
    name: test-vm
    memory: 8192
    cpus: 4
    disk: /path/to/disk.qcow2
    kernel: /path/to/kernel
    initrd: /path/to/initrd
    cmdline: console=ttyS0
    qemu_binary: /usr/bin/qemu-system-x86_64
  confidential:
    technology: amd-sev-snp
    firmware: /path/to/OVMF.fd
  gpu:
    enable: true
    vendor: nvidia
    devices:
      - "0000:0a:00.0"
      - "0000:0b:00.0"

numa:
  enable: true
  nodes:
    - node_id: 0
      cpus: [0, 1, 2, 3]
      memory_gb: 32
      gpus: ["0000:0a:00.0"]

hugepages:
  enable: true
  page_size_kb: 2048
  num_pages: 1024
"#;
        
        let config: EnclaveConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.general.name, "test-enclave");
        assert_eq!(config.general.backend, BackendType::Qemu);
        
        let qemu = config.qemu.unwrap();
        assert_eq!(qemu.vm.cpus, 4);
        assert_eq!(qemu.confidential.technology, TeeType::AmdSevSnp);
        
        let gpu = qemu.gpu.unwrap();
        assert!(gpu.enable);
        assert_eq!(gpu.vendor, GpuVendor::Nvidia);
        assert_eq!(gpu.devices.len(), 2);
    }
}