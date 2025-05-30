use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::hash::hasher;
use crate::monitor_module::state::FileInfo;
use crate::monitor_module::fs_utils::{self, walk_directory};
use dashmap::DashMap;
use sha3::Digest;
use fs_utils::is_directory;
use async_nats::jetstream::kv::Store as KvStore;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct HashInfo {
    pub ongoing_tasks: Arc<Mutex<HashMap<String, JoinHandle<io::Result<Vec<u8>>>>>>,
    hash_results: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    kv_store: Arc<KvStore>,
}

impl HashInfo{
    pub fn new(kv_store: KvStore) -> Self {
        Self {
            ongoing_tasks: Arc::new(Mutex::new(HashMap::new())),
            hash_results: Arc::new(Mutex::new(HashMap::new())),
            kv_store: Arc::new(kv_store),
        }
        
    }

    pub async fn add_hash_entry(&self, file_path: &String, hash: Vec<u8>) -> io::Result<()> {
        let mut results_guard = self.hash_results.lock().await;
        let entry_hashes = results_guard.entry(file_path.clone()).or_insert_with(Vec::new);
        entry_hashes.push(hash.clone());
        drop(results_guard);

        // Update the KV Store
        if let Err(e) = self.kv_store.put(&file_path, Bytes::from(hash)).await {
            eprintln!("Failed to put hash for {} to KV store: {}", file_path, e);
            return Err(io::Error::new(io::ErrorKind::Other, format!("KV put error: {}", e)));
        }
        Ok(())
    }

    pub async fn remove_hash_entry(&self, file_path: &String) -> io::Result<()> {
        let mut results_guard = self.hash_results.lock().await;
        results_guard.remove(file_path);
        drop(results_guard);

        // Update the KV Store
        if let Err(e) = self.kv_store.delete(&file_path).await {
           eprintln!("Failed to delete hash for {} from KV store: {}", file_path, e);
           return Err(io::Error::new(io::ErrorKind::Other, format!("KV delete error: {}", e)));
       }
       Ok(())
    }

    pub async fn get_hash(&self, file_path: &String) -> io::Result<(Vec<u8>)> {
        // Check if hashing task is pending
        let tasks_guard = self.ongoing_tasks.lock().await;
        if tasks_guard.contains_key(file_path) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Hashing for {} is yet to complete", file_path)));
        }

        let results_guard = self.hash_results.lock().await;
        let hash_vector = results_guard.get(file_path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("No hashes recorded for {}", file_path)))?;

        // This version matching might be too restrictive here.
        // Removed for now
        // let version = file_info.version as usize;
        // if hash_vector.len() != version {
        //     return Err(io::Error::new(io::ErrorKind::NotFound, "Latest hash is not available"));
        // }

        hash_vector.last()
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("No hash available for {}", file_path)))
    }

    pub async fn rename_hash_entry(&self, from_path: &String, to_path: &String) -> io::Result<()> {
        let mut results_guard = self.hash_results.lock().await;
        if let Some(hash_history) = results_guard.remove(from_path) {
            let latest_hash_opt = hash_history.last().cloned();
            results_guard.insert(to_path.clone(), hash_history);
            drop(results_guard);

            // Update the KV Store
            if let Some(latest_hash) = latest_hash_opt {
                if let Err(e) = self.kv_store.delete(&from_path).await {
                    eprintln!("Failed to delete old hash for {} from KV store after rename: {}", from_path, e);
                }
                if let Err(e) = self.kv_store.put(&to_path, Bytes::from(latest_hash)).await {
                    eprintln!("Failed to put renamed hash for {} to KV store: {}", to_path, e);
                    return Err(io::Error::new(io::ErrorKind::Other, format!("KV put error (rename): {}", e)));
                }
            }
        } else {
            drop(results_guard);
        }
        Ok(())
    }
}

pub async fn perform_file_hashing(path: String, hash_info_arc: Arc<HashInfo>) {
    let file_path = path;
    let handle = tokio::task::spawn_blocking({
        let file_path = file_path.clone();
        move || hasher::hash_file(&file_path)
    });

    hash_info_arc.ongoing_tasks.lock().await.insert(file_path.clone(), handle);

    let ongoing_tasks = Arc::clone(&hash_info_arc.ongoing_tasks);
    let hash_info_for_result = Arc::clone(&hash_info_arc);
    let file_path_clone = file_path.clone();
    
    tokio::spawn(async move {
        let task_result = {
            let mut tasks_guard = ongoing_tasks.lock().await;
            if let Some(handle_ref) = tasks_guard.get_mut(&file_path_clone) {
                Some(async { handle_ref.await }.await)
            } else {
                None
            }
        };

        if let Some(result) = task_result {
            match result {
                Ok(Ok(hash_bytes)) => {
                    if let Err(e) = hash_info_for_result.add_hash_entry(&file_path_clone, hash_bytes).await {
                        eprintln!("Error adding hash entry for {}: {}", &file_path_clone, e);
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Error hashing file: {}", e);
                }
                Err(e) => {
                    eprintln!("Task panicked while hashing file: {}", e);
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

// This checks superset of conditions tested by 
// ready-handler in web-ra-srv. This should be 
// adapted during integration.
pub async fn retrieve_file_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) -> io::Result<Vec<u8>> {
    let file_info = file_infos.get(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("File {} is not tracked", path)))?;


    if file_info.state != crate::monitor_module::state::FileState::Closed {
        return Err(io::Error::new(io::ErrorKind::Other, format!("File {} is yet to be closed", path)));
    }

    // Check if hashing task is pending
    let tasks = hash_info.ongoing_tasks.lock().await;
    if tasks.contains_key(path) {
        return Err(io::Error::new(io::ErrorKind::Other, format!("Hashing for {} is yet to complete", path)));
    }
    // The code below needs to be updated to be useful opaquely with storage method.
    // Better way would be to add a method that retrieve's a file's hash for HashInfo struct directly.
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