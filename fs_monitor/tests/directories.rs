use fs_monitor::monitor_module::{debounced_watcher::setup_debounced_watcher, ignore::IgnoreList, state::{ FileInfo, FileState}, fs_utils::handle_path};
use fs_monitor::hash::storage::{HashInfo, retrieve_hash};
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


// Create a standardized hierarchical directory structure
/*
test_dir_XXX/
├── dir_a/
│   ├── file_XXX.txt
│   └── subdir_2/
│       └── file_XXX.txt
└── dir_b/
    ├── file_XXX.txt
    └── subdir_1/
        └── file_XXX.txt
*/

fn create_test_directory_hierarchy(base_path: &Path) -> Result<(String, Vec<String>), Box<dyn std::error::Error>> {
    // Create a root directory with unique name
    let root_dir_name = format!("test_dir_{}", Uuid::new_v4());
    let root_path = base_path.join(&root_dir_name);
    std::fs::create_dir_all(&root_path)?;
    
    // Small delay after directory creation
    std::thread::sleep(Duration::from_millis(100));
    
    // Keep track of all files created
    let mut all_files = Vec::new();
    
    // Level 1 directories
    let level1_dirs = ["dir_a", "dir_b"];
    for dir in &level1_dirs {
        let dir_path = root_path.join(dir);
        std::fs::create_dir_all(&dir_path)?;
        std::thread::sleep(Duration::from_millis(100));
        
        // Create files in level 1 directories
        let file_path = dir_path.join(format!("file_{}.txt", Uuid::new_v4()));
        {
            let mut file = File::create(&file_path)?;
            file.write_all(format!("Content in {}", dir).as_bytes())?;
            file.flush()?;
        }
        std::thread::sleep(Duration::from_millis(200));
        all_files.push(file_path.to_string_lossy().to_string());
        
        let subdir_path = dir_path.join("subdir_1");
        std::fs::create_dir_all(&subdir_path)?;
        std::thread::sleep(Duration::from_millis(100));
        
        // Create file in level 2
        let file_path = subdir_path.join(format!("file_{}.txt", Uuid::new_v4()));
        {
            let mut file = File::create(&file_path)?;
            file.write_all(format!("Content in {}/subdir_1", dir).as_bytes())?;
            file.flush()?;
        }
        std::thread::sleep(Duration::from_millis(200));
        all_files.push(file_path.to_string_lossy().to_string());
    }
    
    // Allow time for all file events to be processed
    std::thread::sleep(Duration::from_millis(500));
    
    Ok((root_path.to_string_lossy().to_string(), all_files))
}

#[tokio::test]
async fn directory_deletion() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create hierarchical directory
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;
    
    // Wait for all files to be detected
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, Some(FileState::Closed), 4000).await);
    }
    
    // Delete the directory
    std::fs::remove_dir_all(&root_dir)?;
    
    // Verify files are removed from tracking
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, None, 4000).await);
    }
    
    Ok(())
}

#[tokio::test]
async fn directory_rename_simple() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create hierarchical directory
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;
    
    // Wait for all files to be detected
    let mut file_hashes = Vec::new();
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, Some(FileState::Closed), 4000).await);
        
        // Store hash for later comparison
        let hash = retrieve_hash(&normalized_path, &file_infos, &hash_infos).await.unwrap();
        file_hashes.push((normalized_path, hash));
    }
    
    // Rename the directory
    let new_dir_name = format!("renamed_dir_{}", Uuid::new_v4());
    let new_dir_path = Path::new(&new_dir_name);
    std::fs::rename(&root_dir, new_dir_path)?;
    
    // Verify old paths are gone
    for (path, _) in &file_hashes {
        assert!(wait_for_file_state(&file_infos, path, None, 4000).await);
    }
    
    // Verify new paths exist with same hashes
    for (i, file_path) in file_paths.iter().enumerate() {
        let new_path = file_path.replace(&root_dir, &new_dir_name);
        let normalized_new_path = handle_path(&new_path);
        
        // Wait for file to be detected at new location
        assert!(wait_for_file_state(&file_infos, &normalized_new_path, Some(FileState::Closed), 4000).await);
        
        // Check hash is the same
        let new_hash = retrieve_hash(&normalized_new_path, &file_infos, &hash_infos).await.unwrap();
        assert_eq!(file_hashes[i].1, new_hash);
    }
    
    // Clean up
    std::fs::remove_dir_all(new_dir_path)?;
    
    Ok(())
}

#[tokio::test]
async fn directory_rename_to_watched() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create directory in unwatched location
    let (root_dir, file_paths) = create_test_directory_hierarchy(std::env::temp_dir().as_path())?;
    
    // Move directory to watched location
    let target_dir = format!("moved_dir_{}", Uuid::new_v4());
    std::fs::rename(&root_dir, &target_dir)?;
    
    // Verify files are detected in new location
    for file_path in &file_paths {
        let new_path = file_path.replace(&root_dir, &target_dir);
        let normalized_new_path = handle_path(&new_path);
        
        assert!(wait_for_file_state(&file_infos, &normalized_new_path, Some(FileState::Closed), 4000).await);
    }
    
    // Clean up
    std::fs::remove_dir_all(&target_dir)?;
    
    Ok(())
}

#[tokio::test]
async fn directory_rename_to_unwatched() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create directory in watched location
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;
    
    // Wait for all files to be detected
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, Some(FileState::Closed), 4000).await);
    }
    
    // Move directory to unwatched location
    let target_dir = std::env::temp_dir().join(format!("moved_unwatched_{}", Uuid::new_v4()));
    std::fs::rename(&root_dir, &target_dir)?;
    
    // Verify files are no longer tracked
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, None, 4000).await);
    }
    
    // Clean up
    std::fs::remove_dir_all(target_dir)?;
    
    Ok(())
}

#[tokio::test]
async fn directory_rename_to_ignored() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create directory in watched location
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;
    
    // Wait for all files to be detected
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, Some(FileState::Closed), 4000).await);
    }
    
    // Rename directory to an ignored path (using tmp_* pattern from fs_ignore)
    let new_dir_name = format!("tmp_{}", Uuid::new_v4());
    let new_dir_path = Path::new(&new_dir_name);
    std::fs::rename(&root_dir, new_dir_path)?;
    
    // Verify files are no longer tracked (should be removed because path is now ignored)
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, None, 4000).await);
    }
    
    // Clean up
    std::fs::remove_dir_all(new_dir_path)?;
    
    Ok(())
}

#[tokio::test]
async fn directory_rename_from_ignored() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create a directory with an ignored name pattern
    let ignored_dir_name = format!("tmp_{}", Uuid::new_v4());
    let ignored_dir_path = Path::new(&ignored_dir_name);
    std::fs::create_dir_all(ignored_dir_path)?;
    
    // Create the directory hierarchy inside the ignored directory
    let (_, file_paths) = create_test_directory_hierarchy(ignored_dir_path)?;
    
    // Verify files in ignored directory are not tracked
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, None, 4000).await);
    }
    
    // Rename the directory to non-ignored name
    let new_dir_name = format!("watched_dir_{}", Uuid::new_v4());
    let new_dir_path = Path::new(&new_dir_name);
    std::fs::rename(ignored_dir_path, new_dir_path)?;
    
    // Verify files are now tracked after moving to non-ignored directory
    for file_path in &file_paths {
        let new_path = file_path.replace(&ignored_dir_name, &new_dir_name);
        let normalized_new_path = handle_path(&new_path);
        
        assert!(wait_for_file_state(&file_infos, &normalized_new_path, Some(FileState::Closed), 4000).await);
    }
    
    // Clean up
    std::fs::remove_dir_all(new_dir_path)?;
    
    Ok(())
}

#[tokio::test]
async fn directory_rename_to_dotcache() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create directory in watched location
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;
    
    // Wait for all files to be detected
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, Some(FileState::Closed), 4000).await);
    }
    
    let cache_dir = ".cache";
    std::fs::create_dir_all(cache_dir)?; // Ensure .cache exists
    let new_dir_path = Path::new(cache_dir).join(format!("cache_content_{}", Uuid::new_v4()));
    std::fs::rename(&root_dir, &new_dir_path)?;
    
    // Verify files are no longer tracked
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&file_infos, &normalized_path, None, 4000).await);
    }
    
    // Clean up
    std::fs::remove_dir_all(cache_dir)?;
    
    Ok(())
}
