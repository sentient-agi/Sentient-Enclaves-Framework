use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnclaveConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    
    #[serde(default)]
    pub qemu: Option<QemuConfig>,
    
    #[serde(default)]
    pub nitro: Option<NitroConfig>,
    
    #[serde(default)]
    pub numa: Option<NumaConfig>,
    
    #[serde(default)]
    pub hugepages: Option<HugepagesConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub name: String,
    
    #[serde(default = "default_backend")]
    pub backend: BackendType,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            name: "default-enclave".to_string(),
            backend: BackendType::Qemu,
        }
    }
}

fn default_backend() -> BackendType {
    BackendType::Qemu
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    Qemu,
    Nitro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QemuConfig {
    pub vm: VmConfig,
    pub confidential: ConfidentialConfig,
    
    #[serde(default)]
    pub gpu: Option<GpuConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub name: String,
    pub memory: u64,
    pub cpus: u32,
    pub disk: PathBuf,
    pub kernel: PathBuf,
    pub initrd: PathBuf,
    pub cmdline: String,
    
    #[serde(default)]
    pub qemu_binary: String,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            name: "default-vm".to_string(),
            memory: 4096,
            cpus: 4,
            disk: PathBuf::from("/path/to/disk.qcow2"),
            kernel: PathBuf::from("/path/to/kernel"),
            initrd: PathBuf::from("/path/to/initrd"),
            cmdline: "console=ttyS0".to_string(),
            qemu_binary: "/usr/bin/qemu-system-x86_64".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialConfig {
    pub technology: TeeType,
    
    #[serde(default)]
    pub firmware: String,
    
    #[serde(default)]
    pub id_key: Option<PathBuf>,
    
    #[serde(default)]
    pub measurement: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TeeType {
    IntelTdx,
    AmdSev,
    AmdSevSnp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    pub enable: bool,
    pub vendor: GpuVendor,
    
    /// Multiple GPU BDF addresses for multi-GPU passthrough
    pub devices: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GpuVendor {
    Nvidia,
    Amd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NitroConfig {
    pub enclave_name: String,
    pub cpu_count: u32,
    pub memory_mib: u64,
    
    /// Enclave Image File (EIF)
    pub eif_path: PathBuf,
    
    /// VSock communication settings
    pub vsock: VsockConfig,
    
    #[serde(default)]
    pub debug_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VsockConfig {
    /// Context ID for VSock
    pub cid: u32,
    
    /// Port for VSock communication
    pub port: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumaConfig {
    pub enable: bool,
    
    /// NUMA node configurations
    pub nodes: Vec<NumaNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumaNode {
    pub node_id: u32,
    pub cpus: Vec<u32>,
    pub memory_gb: u64,
    
    #[serde(default)]
    pub gpus: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HugepagesConfig {
    pub enable: bool,
    
    /// Hugepage size in KB (2048 for 2MB, 1048576 for 1GB)
    pub page_size_kb: u64,
    
    /// Number of hugepages to allocate
    pub num_pages: u64,
    
    #[serde(default)]
    pub mount_point: Option<PathBuf>,
}