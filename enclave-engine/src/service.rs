use crate::config::{EnclaveConfig, BackendType};
use crate::error::{EnclaveError, Result};
use crate::backends::{qemu::QemuBackend, nitro::NitroBackend};
use crate::provisioners::{numa::NumaManager, hugepages::HugepagesManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::info;

#[derive(Clone)]
pub struct EnclaveService {
    qemu_backend: Arc<QemuBackend>,
    nitro_backend: Arc<NitroBackend>,
    numa_manager: Arc<NumaManager>,
    hugepages_manager: Arc<HugepagesManager>,
    enclaves: Arc<RwLock<HashMap<String, EnclaveInstance>>>,
}

#[derive(Debug, Clone)]
pub struct EnclaveInstance {
    pub id: String,
    pub name: String,
    pub backend: BackendType,
    pub status: EnclaveStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnclaveStatus {
    Provisioning,
    Running,
    Stopped,
    Failed,
}

impl EnclaveService {
    pub fn new() -> Self {
        Self {
            qemu_backend: Arc::new(QemuBackend::new()),
            nitro_backend: Arc::new(NitroBackend::new()),
            numa_manager: Arc::new(NumaManager::new()),
            hugepages_manager: Arc::new(HugepagesManager::new()),
            enclaves: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn provision(&self, config: EnclaveConfig) -> Result<String> {
        let enclave_id = Uuid::new_v4().to_string();
        let enclave_name = config.general.name.clone();
        
        info!("Provisioning enclave: {} ({})", enclave_name, enclave_id);
        
        // Create enclave instance
        let instance = EnclaveInstance {
            id: enclave_id.clone(),
            name: enclave_name.clone(),
            backend: config.general.backend,
            status: EnclaveStatus::Provisioning,
        };
        
        // Register enclave
        {
            let mut enclaves = self.enclaves.write().await;
            if enclaves.contains_key(&enclave_name) {
                return Err(EnclaveError::AlreadyExists(enclave_name));
            }
            enclaves.insert(enclave_name.clone(), instance);
        }
        
        // Configure NUMA if specified
        if let Some(numa_config) = &config.numa {
            self.numa_manager.configure(numa_config).await?;
        }
        
        // Configure hugepages if specified
        if let Some(hugepages_config) = &config.hugepages {
            self.hugepages_manager.configure(hugepages_config).await?;
        }
        
        // Provision based on backend
        let result = match config.general.backend {
            BackendType::Qemu => {
                let qemu_config = config.qemu
                    .ok_or_else(|| EnclaveError::Config("QEMU config required".to_string()))?;
                self.qemu_backend.provision(&qemu_config).await
            }
            BackendType::Nitro => {
                let nitro_config = config.nitro
                    .ok_or_else(|| EnclaveError::Config("Nitro config required".to_string()))?;
                
                // Allocate resources first
                self.nitro_backend.allocate_resources(&nitro_config).await?;
                
                // Provision enclave
                self.nitro_backend.provision(&nitro_config).await
            }
        };
        
        // Update status
        {
            let mut enclaves = self.enclaves.write().await;
            if let Some(instance) = enclaves.get_mut(&enclave_name) {
                instance.status = match result {
                    Ok(_) => EnclaveStatus::Running,
                    Err(_) => EnclaveStatus::Failed,
                };
            }
        }
        
        result?;
        
        info!("Enclave {} provisioned successfully", enclave_name);
        Ok(enclave_id)
    }
    
    pub async fn stop(&self, name: &str) -> Result<()> {
        info!("Stopping enclave: {}", name);
        
        let backend = {
            let enclaves = self.enclaves.read().await;
            let instance = enclaves.get(name)
                .ok_or_else(|| EnclaveError::NotFound(name.to_string()))?;
            instance.backend
        };
        
        // Stop based on backend
        match backend {
            BackendType::Qemu => {
                self.qemu_backend.stop(name).await?;
            }
            BackendType::Nitro => {
                self.nitro_backend.stop(name).await?;
            }
        }
        
        // Update status
        {
            let mut enclaves = self.enclaves.write().await;
            if let Some(instance) = enclaves.get_mut(name) {
                instance.status = EnclaveStatus::Stopped;
            }
        }
        
        info!("Enclave {} stopped", name);
        Ok(())
    }
    
    pub async fn delete(&self, name: &str) -> Result<()> {
        info!("Deleting enclave: {}", name);
        
        // Stop if running
        if let Ok(_) = self.stop(name).await {
            info!("Enclave {} stopped before deletion", name);
        }
        
        // Remove from registry
        {
            let mut enclaves = self.enclaves.write().await;
            enclaves.remove(name)
                .ok_or_else(|| EnclaveError::NotFound(name.to_string()))?;
        }
        
        info!("Enclave {} deleted", name);
        Ok(())
    }
    
    pub async fn status(&self, name: &str) -> Result<EnclaveInstance> {
        let enclaves = self.enclaves.read().await;
        enclaves.get(name)
            .cloned()
            .ok_or_else(|| EnclaveError::NotFound(name.to_string()))
    }
    
    pub async fn list(&self) -> Result<Vec<EnclaveInstance>> {
        let enclaves = self.enclaves.read().await;
        Ok(enclaves.values().cloned().collect())
    }
    
    pub async fn get_numa_info(&self) -> Result<String> {
        self.numa_manager.show_numa_info().await
    }
    
    pub async fn get_hugepages_info(&self) -> Result<String> {
        self.hugepages_manager.show_hugepage_info().await
    }
}