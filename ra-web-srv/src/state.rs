//! Application state types

use async_nats::client::Client as NATSClient;
use async_nats::jetstream::kv::Store as KVStore;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use vrf::openssl::CipherSuite;

/// Application runtime state
#[derive(Default, Debug, Clone)]
pub struct AppState {
    pub nsm_fd: i32,
    pub sk4proofs: Vec<u8>,
    pub sk4docs: Vec<u8>,
    pub vrf_cipher_suite: CipherSuite,
    pub nats_client: Option<NATSClient>,
    pub storage: Option<KVStore>,
}

/// Application cache for attestation data
#[derive(Default, Debug, Clone)]
pub struct AppCache {
    pub att_data: HashMap<String, AttData>,
}

/// Attestation data for a file
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AttData {
    pub file_path: String,
    pub sha3_hash: String,
    pub vrf_proof: String,
    pub vrf_cipher_suite: CipherSuite,
    pub att_doc: Vec<u8>,
}

/// Attestation user data embedded in attestation documents
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AttUserData {
    pub file_path: String,
    pub sha3_hash: String,
    pub vrf_proof: String,
    pub vrf_cipher_suite: CipherSuite,
}

/// Attestation proof data for VRF operations
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AttProofData {
    pub file_path: String,
    pub sha3_hash: String,
}

/// Server state combining all state components
#[derive(Default, Debug, Clone)]
pub struct ServerState {
    pub tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<std::io::Result<Vec<u8>>>>>>,
    pub results: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    pub app_state: Arc<RwLock<AppState>>,
    pub app_cache: Arc<RwLock<AppCache>>,
}

impl ServerState {
    /// Create a new server state
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            results: Arc::new(Mutex::new(HashMap::new())),
            app_state: Arc::new(RwLock::new(AppState::default())),
            app_cache: Arc::new(RwLock::new(AppCache::default())),
        }
    }
}
