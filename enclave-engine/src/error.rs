use thiserror::Error;

#[derive(Error, Debug)]
pub enum EnclaveError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("QEMU provisioning error: {0}")]
    Qemu(String),
    
    #[error("Nitro provisioning error: {0}")]
    Nitro(String),
    
    #[error("NUMA configuration error: {0}")]
    Numa(String),
    
    #[error("Hugepages configuration error: {0}")]
    Hugepages(String),
    
    #[error("GPU passthrough error: {0}")]
    Gpu(String),
    
    #[error("Command execution error: {0}")]
    CommandExec(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    
    #[error("Enclave not found: {0}")]
    NotFound(String),
    
    #[error("Enclave already exists: {0}")]
    AlreadyExists(String),
}

pub type Result<T> = std::result::Result<T, EnclaveError>;