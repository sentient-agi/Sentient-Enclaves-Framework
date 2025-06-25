pub mod hasher;
pub mod storage;
use crate::monitor_module::state::FileInfo;
use crate::monitor_module::fs_utils::{walk_directory, is_directory};
use dashmap::DashMap;
use sha3::Digest;
use storage::HashInfo;
use std::sync::Arc;
use std::io;

pub async fn retrieve_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) -> io::Result<String> {
    if is_directory(path) {
        let files = walk_directory(path)?;

        // Filter files to only include those that are being tracked
        let tracked_files: Vec<String> = files
            .into_iter()
            .filter(|file_path| file_infos.contains_key(file_path))
            .collect();

        // Sort files for consistent hash results
        let mut sorted_files = tracked_files;
        sorted_files.sort();

        println!("Processing directory: {} with {} tracked files", path, sorted_files.len());

        let mut dir_hasher = sha3::Sha3_512::new();
        for file_path in sorted_files {
            println!("Retrieving hash for: {}", file_path);
            let file_hash = retrieve_file_hash(&file_path, file_infos, hash_info).await?;
            dir_hasher.update(file_hash);
        }

        Ok(hasher::bytes_to_hex(&dir_hasher.finalize()))
    } else {
        let hash_bytes = retrieve_file_hash(path, file_infos, hash_info).await?;
        Ok(hasher::bytes_to_hex(&hash_bytes))
    }
}

// This checks superset of conditions tested by
// ready-handler in web-ra-srv. This should be
// adapted during integration.
async fn retrieve_file_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) -> io::Result<Vec<u8>> {
    // Checks if the fs_monitor tracks the file before trying to retrieve the hash.
    let file_info = file_infos.get(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("File {} is not tracked", path)))?;

    // This is the case when the file is probably being modified.
    // Retrieving hash in that state would provide stale hash.
    // So, instead return error.
    if file_info.state != crate::monitor_module::state::FileState::Closed {
        return Err(io::Error::new(io::ErrorKind::Other, format!("File {} is yet to be closed", path)));
    }

    // Retrieve file hash from hash_info struct
    let file_path = &path.to_string();
    match hash_info.get_hash_entry(file_path).await {
        Ok(hash) => Ok(hash),
        Err(e) => Err(e),
    }
}
