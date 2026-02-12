//! File hashing utilities

use crate::errors::AppResult;
use sha3::{Digest, Sha3_512};
use std::io::Read;
use tracing::{debug, error};

/// Hash a file using SHA3-512
pub fn hash_file(file_path: &str) -> std::io::Result<Vec<u8>> {
    debug!("Hashing file: {}", file_path);

    let mut file = std::fs::File::open(file_path)?;
    let mut hasher = Sha3_512::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize().to_vec();
    debug!("File '{}' hashed: {} bytes -> {} bytes hash", file_path, file.metadata()?.len(), result.len());
    Ok(result)
}
