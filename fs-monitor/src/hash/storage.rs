use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use crate::hash::hasher;
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
        // Update the HashMap
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
        // Update the HashMap
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

    pub async fn get_hash_entry(&self, file_path: &String) -> io::Result<Vec<u8>> {
        // Check if hashing task is pending
        let tasks_guard = self.ongoing_tasks.lock().await;
        if tasks_guard.contains_key(file_path) {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Hashing for {} is yet to complete", file_path)));
        }

        // Use KV store to obtain the hash
        let hash = self.kv_store.get(file_path).await;
        match hash {
            Ok(Some(bytes)) => {
                return Ok(bytes.into());
            }
            Ok(None) => {
                return Err(io::Error::new(io::ErrorKind::NotFound, format!("No hash available for {}", file_path)));
            }
            Err(e) => {
                eprintln!("Failed to get hash for {} from KV store: {}", file_path, e);
                return Err(io::Error::new(io::ErrorKind::Other, format!("KV get error: {}", e)));
            }
        }
    }

    pub async fn rename_hash_entry(&self, from_path: &String, to_path: &String) -> io::Result<()> {
        // Update the HashMap
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

    let hash_info_for_result = Arc::clone(&hash_info_arc);
    let file_path_clone = file_path.clone();

    tokio::spawn(async move {
        let task_result = {
            let mut tasks_guard = hash_info_for_result.ongoing_tasks.lock().await;
            if let Some(handle_ref) = tasks_guard.get_mut(&file_path_clone) {
                Some(async { handle_ref.await }.await)
            } else {
                None
            }
        };

        // If hashing has finished, add it to the storage
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
            let mut tasks = hash_info_for_result.ongoing_tasks.lock().await;
            tasks.remove(&file_path_clone);
        }
    });
}
