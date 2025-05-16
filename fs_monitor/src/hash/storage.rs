use std::collections::HashMap;
use std::fmt::format;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::hash::hasher;
use crate::fs_ops::state::FileInfo;
use crate::fs_ops::fs_utils;
use dashmap::DashMap;
use sha3::Digest;
use fs_utils::is_directory;

#[derive(Debug, Clone)]
pub struct HashInfo {
    pub ongoing_tasks: Arc<Mutex<HashMap<String, JoinHandle<io::Result<Vec<u8>>>>>>,
    pub hash_results: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
}

pub async fn perform_file_hashing(path: String, hash_info: Arc<HashInfo>) {
    let file_path = path;
    let handle = tokio::task::spawn_blocking({
        let file_path = file_path.clone();
        move || hasher::hash_file(&file_path)
    });

    hash_info.ongoing_tasks.lock().await.insert(file_path.clone(), handle);

    let ongoing_tasks = Arc::clone(&hash_info.ongoing_tasks);
    let hash_results = Arc::clone(&hash_info.hash_results);
    let file_path_clone = file_path.clone();
    
    tokio::spawn(async move {
        let task_result = {
            let mut tasks = ongoing_tasks.lock().await;
            if let Some(handle) = tasks.get_mut(&file_path_clone) {
                Some(async { handle.await }.await)
            } else {
                None
            }
        };

        if let Some(result) = task_result {
            match result {
                Ok(Ok(hash)) => {
                    let mut results = hash_results.lock().await;
                    let mut hashes = results.get(&file_path_clone).unwrap_or(&Vec::new()).clone();
                    hashes.push(hash);
                    results.insert(file_path_clone.clone(), hashes);
                }
                Ok(Err(e)) => {
                    eprintln!("Error hashing file: {}", e);
                }
                Err(e) => {
                    eprintln!("Task panicked: {}", e);
                }
            }
            // Remove the task from HashMap after awaiting it (after it completes)
            let mut tasks = ongoing_tasks.lock().await;
            tasks.remove(&file_path_clone);
        }
    });
}


pub async fn retrieve_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) -> io::Result<String> {
    if is_directory(path) {
        let mut files = Vec::new();
        
        // Use proper filesystem traversal instead of string prefix matching
        let dir_path = std::path::Path::new(path);
        fs_utils::collect_files_recursively(dir_path, &mut files)?;
        
        // Filter files to only include those that are being tracked
        let tracked_files: Vec<String> = files.into_iter()
            .filter(|file_path| file_infos.contains_key(file_path))
            .collect();
        
        // Sort files for consistent hash results
        let mut sorted_files = tracked_files;
        sorted_files.sort();

        println!("Processing directory: {} with {} tracked files", path, sorted_files.len());
        
        let mut dir_hasher = sha3::Sha3_512::new();
        for file_path in sorted_files {
            println!("Calculating hash for: {}", file_path);
            let file_hash = retrieve_file_hash(&file_path, file_infos, hash_info).await?;
            dir_hasher.update(file_hash);
        }

        Ok(hasher::bytes_to_hex(&dir_hasher.finalize()))
    } else {
        let hash_bytes = retrieve_file_hash(path, file_infos, hash_info).await?;
        Ok(hasher::bytes_to_hex(&hash_bytes))
    }
}

pub async fn retrieve_file_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) -> io::Result<Vec<u8>> {
    let file_info = file_infos.get(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("File {} is not tracked", path)))?;


    if file_info.state != crate::fs_ops::state::FileState::Closed {
        return Err(io::Error::new(io::ErrorKind::Other, format!("File {} is yet to be closed", path)));
    }

    // Check if hashing task is pending
    let tasks = hash_info.ongoing_tasks.lock().await;
    if tasks.contains_key(path) {
        return Err(io::Error::new(io::ErrorKind::Other, format!("Hashing for {} is yet to complete", path)));
    }
    let results_map = hash_info.hash_results.lock().await;
    let hash_vector = results_map.get(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("No hashes recorded for {}", path)))?;

    // This version matching might be too restrictive here.
    // Removed for now
    // let version = file_info.version as usize;
    // if hash_vector.len() != version {
    //     return Err(io::Error::new(io::ErrorKind::NotFound, "Latest hash is not available"));
    // }

    hash_vector.last()
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("No hash available for {}", path)))
} 

// Lazy cleanup of removed/renamed files

pub async fn hash_cleanup(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: Arc<HashInfo>) {
    let mut hash_results = hash_info.hash_results.lock().await;
    hash_results.remove(path);

    // Remove information about the file
    file_infos.remove(path);
}