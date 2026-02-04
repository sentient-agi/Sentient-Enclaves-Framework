//! Remote Attestation Web Server Library
//!
//! This library provides the core functionality for the remote attestation web server
//! used in Sentient Enclaves Framework.

pub mod attestation;
pub mod cipher;
pub mod config;
pub mod crypto;
pub mod errors;
pub mod handlers;
pub mod hashing;
pub mod nats;
pub mod nsm;
pub mod requests;
pub mod server;
pub mod state;

// Re-export commonly used types
pub use config::{AppConfig, Keys, NATSMQPersistency, Ports};
pub use errors::{AppError, AppResult};
pub use state::{AppCache, AppState, AttData, AttProofData, AttUserData, ServerState};
