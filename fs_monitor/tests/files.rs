use fs_monitor::fs_ops::{watcher::setup_watcher, IgnoreList, state::{ FileInfo, FileState}, fs_utils::handle_path};
use fs_monitor::hash::{storage::{HashInfo, retrieve_hash}, hasher::{hash_file, bytes_to_hex}};
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};
use uuid::Uuid;

use std::path::Path;
use std::sync::Arc;
use dashmap::DashMap;

async fn wait_for_file_state(
    file_infos: &Arc<DashMap<String, FileInfo>>,
    path: &str,
    expected_state: Option<FileState>,
    timeout_ms: u64
) -> bool {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    
    while start.elapsed() < timeout {
        // If expected_state is None, we're waiting for the file to be removed
        if expected_state.is_none() {
            if !file_infos.contains_key(path){
                eprintln!("File no longer exists in map as expected");
                return true;
            }
            else{
                let info = file_infos.get(path).unwrap();
                eprintln!("Found file state: {:?} (expecting None)", info.state);
            }
        } else {
            // Original case - waiting for a specific state
            if let Some(info) = file_infos.get(path) {
                eprintln!("Found file state: {:?} (expecting {:?})", info.state, expected_state);
                if Some(info.state.clone()) == expected_state {
                    return true;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    // Log timeout information
    if expected_state.is_none() {
        eprintln!("Timed out waiting for file to be removed from map");
    } else {
        eprintln!("Timed out waiting for file state: {:?}", expected_state);
    }
    
    false
} 

#[tokio::test]
async fn file_modification_simple() -> notify::Result<()>{
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./empty_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    // Create a temp file
    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Created), 1000).await);

    // Write random data to the file
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();

    file.write_all(b" MORE DATA").unwrap();
    let _ = file.flush();
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Modified), 1000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);
    
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Closed), 1000).await);
    
    // Calculate hash
    let calculated_hash = bytes_to_hex(&hash_file(&file_path).unwrap());
    // Check hashes match
    let retrieved_hash = retrieve_hash(&file_path, &file_infos, &hash_infos).await.unwrap();
    eprintln!("Calculated Hash: {}, Retrieved Hash: {}", calculated_hash, retrieved_hash);
    assert_eq!(calculated_hash, retrieved_hash);
    
    // Cleanup
    eprintln!("Attempting to delete file: {}", file_path);
    std::fs::remove_file(file_path).ok();

    eprintln!("=== Test complete, forcing exit ===");
    Ok(())
}

#[tokio::test]
async fn file_deletion_simple() -> notify::Result<()> {
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./empty_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Created), 1000).await);

    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Modified), 1000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);

    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Closed), 1000).await);

    eprintln!("Attempting to delete file: {}", file_path);
    std::fs::remove_file(file_path.clone()).ok();

    assert!(wait_for_file_state(&file_infos, &file_path, None, 1000).await);

    eprintln!("=== Test complete, forcing exit ===");
    Ok(())

}

// Deleting an empty file
#[tokio::test]
async fn file_deletion_empty() -> notify::Result<()> {
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./empty_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Created), 1000).await);

    eprintln!("Attempting to delete file: {}", file_path);
    std::fs::remove_file(file_path.clone()).ok();
    assert!(wait_for_file_state(&file_infos, &file_path, None, 1000).await);
    Ok(())

}

