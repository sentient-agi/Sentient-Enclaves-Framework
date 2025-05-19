use fs_monitor::monitor_module::{debounced_watcher::setup_debounced_watcher, IgnoreList, state::{ FileInfo, FileState}, fs_utils::handle_path};
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
                // If we find the file but with a different state, log it but keep waiting
                eprintln!("File exists but in different state. Current: {:?}, Expected: {:?}", info.state, expected_state);
            }
        }
        eprintln!("Sleeping");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    // Log timeout information with more details
    if expected_state.is_none() {
        eprintln!("Timed out waiting for file to be removed from map");
    } else {
        eprintln!("Timed out waiting for state: {:?} for file: {:?}", expected_state, path);
    }
    eprintln!("File infos struct: {:?}", file_infos);
    
    false
} 

#[tokio::test]
async fn file_modification_simple() -> Result<(), Box<dyn std::error::Error>>{
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./empty_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_debounced_watcher(
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
    // 
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Created), 4000).await);

    // Write random data to the file
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();

    file.write_all(b" MORE DATA").unwrap();
    let _ = file.flush();
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);
    
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Closed), 4000).await);
    
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
async fn file_deletion_simple() -> Result<(), Box<dyn std::error::Error>> {
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./empty_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_debounced_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Created), 4000).await);

    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);

    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Closed), 4000).await);

    eprintln!("Attempting to delete file: {}", file_path);
    std::fs::remove_file(file_path.clone()).ok();

    assert!(wait_for_file_state(&file_infos, &file_path, None, 4000).await);

    eprintln!("=== Test complete, forcing exit ===");
    Ok(())

}

// Deleting an empty file
#[tokio::test]
async fn file_deletion_empty() -> Result<(), Box<dyn std::error::Error>> {
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./empty_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_debounced_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Created), 4000).await);

    eprintln!("Attempting to delete file: {}", file_path);
    std::fs::remove_file(file_path.clone()).ok();
    assert!(wait_for_file_state(&file_infos, &file_path, None, 4000).await);
    Ok(())

}

#[tokio::test]
async fn file_rename_basic() -> Result<(), Box<dyn std::error::Error>> {
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./empty_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_debounced_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Created), 4000).await);

    // Perform dummy writes
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);

    tokio::time::sleep(Duration::from_secs(2)).await;
    let old_hash = retrieve_hash(&file_path, &file_infos, &hash_infos).await.unwrap();
    
    eprintln!("Attempting to Rename file: {}", file_path);
    let new_file_path = format!("test_{}.txt", Uuid::new_v4());
    std::fs::rename(file_path.clone(), new_file_path.clone())?;
    
    let new_file_path = handle_path(&new_file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, None, 4000).await);
    assert!(wait_for_file_state(&file_infos, &new_file_path, Some(FileState::Closed), 4000).await);
    let new_hash = retrieve_hash(&new_file_path, &file_infos, &hash_infos).await.unwrap();

    eprintln!("Old Hash: {}, New Hash: {}", old_hash, new_hash);
    assert_eq!(old_hash, new_hash);

    std::fs::remove_file(new_file_path)?;
    Ok(())

}

#[tokio::test]
async fn file_rename_to_unwatched() -> Result<(), Box<dyn std::error::Error>> {
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./empty_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_debounced_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Created), 4000).await);

    // Perform dummy writes
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);
    
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Closed), 4000).await);


    // Move the file out of this directory.
    // As the parent directory is unwatched, this
    // effectively moves the file into an unwatched
    // directory.

    let new_path = Path::new("..").join(&file_path);

    std::fs::rename(file_path.clone(), new_path.clone())?;

    assert!(wait_for_file_state(&file_infos, &file_path, None, 4000).await);

    // Cleanup the renamed file
    std::fs::remove_file(new_path)?;

    Ok(())
}

#[tokio::test]
async fn file_rename_from_unwatched() -> Result<(), Box<dyn std::error::Error>> {
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./empty_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_debounced_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    // Create a temporary directory outside the watched path
    let temp_dir = std::env::temp_dir().join(format!("test_dir_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)?;

    // Create a file in the temporary directory
    let file_name = format!("test_{}.txt", Uuid::new_v4());
    let source_path = temp_dir.join(&file_name);
    let mut file = File::create(&source_path)?;
    file.write_all(b"TEST DATA")?;
    file.flush()?;
    drop(file);

    // Move the file into the watched directory
    let target_path = Path::new(".").join(&file_name);
    std::fs::rename(&source_path, &target_path)?;
    eprintln!("Old target path: {:?}", target_path);
    let target_path = target_path.to_str().unwrap();
    let target_path = handle_path(&target_path);
    eprintln!("New target path: {:?}", target_path);

    // Verify the file is detected and processed
    assert!(wait_for_file_state(&file_infos, &target_path, Some(FileState::Closed), 4000).await);

    // Clean up
    std::fs::remove_file(target_path)?;
    std::fs::remove_dir_all(temp_dir)?;

    Ok(())
}

// Ignored and unwatched differ in that ignored
// gives us both the path but we just voluntarily ignore
// one or both the paths.
#[tokio::test]
async fn file_rename_to_ignored() -> Result<(), Box<dyn std::error::Error>>{
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./fs_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_debounced_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Created), 4000).await);

    // Perform dummy writes
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);

    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Rename to an ignored path
    eprintln!("Attempting to Rename file to an ignored path: {}", file_path);
    let new_file_path = format!("tmp_test_{}.txt", Uuid::new_v4());
    std::fs::rename(file_path.clone(), new_file_path.clone())?;
    
    let new_file_path = handle_path(&new_file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, None, 4000).await);
    assert!(wait_for_file_state(&file_infos, &new_file_path, None, 4000).await);

    std::fs::remove_file(new_file_path)?;
    Ok(())
}

#[tokio::test]
async fn file_rename_from_ignored() -> Result<(), Box<dyn std::error::Error>>{
    let watch_path = Path::new(".");
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./fs_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });

    let _watcher = setup_debounced_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    let file_path = format!("tmp_test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&file_infos, &file_path, None, 4000).await);

    
    let new_file_path = format!("test_{}.txt", Uuid::new_v4());
    eprintln!("Attempting to Rename file from an ignored path: {} to: {}", file_path, new_file_path);

    let new_file_path = handle_path(&new_file_path);
    std::fs::rename(file_path.clone(), new_file_path.clone())?;
    assert!(wait_for_file_state(&file_infos, &new_file_path, Some(FileState::Created), 4000).await);


    // Perform dummy write
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&file_infos, &new_file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(wait_for_file_state(&file_infos, &new_file_path, Some(FileState::Closed), 4000).await);

    std::fs::remove_file(new_file_path)?;
    Ok(())
}