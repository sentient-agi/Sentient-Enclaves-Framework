use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::hash::hasher;
use crate::fs_ops::state::FileInfo;
use dashmap::DashMap;
use sha3::Digest;

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

pub async fn remove_stale_tasks(path: String, hash_info: Arc<HashInfo>) -> io::Result<()> {
    // Get the lock on ongoing_tasks
    let handle_opt = {
        let mut tasks = hash_info.ongoing_tasks.lock().await;
        tasks.remove(&path)
    };
    
    // Handle the task if it exists
    if let Some(handle) = handle_opt {
        // Abort the task to prevent it from continuing to run
        handle.abort();
        
        // Wait for the task to complete abort to ensure resources are freed
        match handle.await {
            Ok(_) => {
                // Task completed before we could abort it
                eprintln!("Task for {} completed before abort", path);
            },
            Err(e) if e.is_cancelled() => {
                // Task was successfully cancelled
                eprintln!("Successfully aborted hashing task for: {}", path);
            },
            Err(e) => {
                // Task panicked or had another error
                eprintln!("Error while aborting task for {}: {}", path, e);
                return Err(io::Error::new(io::ErrorKind::Other, 
                    format!("Failed to abort task properly: {}", e)));
            }
        }
    }
    
    // Also clean up any partial results in the hash_results map
    // This prevents stale data from being accessed later
    {
        let mut results = hash_info.hash_results.lock().await;
        results.remove(&path);
    }
    
    Ok(())
}

pub async fn retrieve_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) -> io::Result<String> {
    let metadata = std::fs::metadata(path).map_err(|e| {
        io::Error::new(e.kind(), format!("Failed to get metadata for '{}': {}", path, e))
    })?;

    if metadata.is_dir() {
        let mut files = Vec::new();
        for entry in file_infos.iter() {
            let file_path = entry.key();
            if file_path.starts_with(path) && !file_path.ends_with('/') {
                files.push(file_path.clone());
            }
        }
        
        // Sort files for consistent hash results
        files.sort();

        let mut dir_hasher = sha3::Sha3_512::new();
        for file_path in files {
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
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "File not tracked"))?;

    if file_info.state != crate::fs_ops::state::FileState::Closed {
        return Err(io::Error::new(io::ErrorKind::Other, "File is yet to be closed"));
    }

    let results_map = hash_info.hash_results.lock().await;
    let hash_vector = results_map.get(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No hashes recorded"))?;

    let version = file_info.version as usize;
    if hash_vector.len() != version {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Latest hash is not available"));
    }

    hash_vector.last()
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No hash available"))
} 