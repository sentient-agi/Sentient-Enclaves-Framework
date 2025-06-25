// Common test utilities
use fs_monitor::monitor_module::{debounced_watcher::setup_debounced_watcher, ignore::IgnoreList, state::{FileInfo, FileState}, fs_utils::set_watch_path};
use fs_monitor::hash::storage::HashInfo;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::Write;
use dashmap::DashMap;
use async_nats::jetstream::kv::Store as KvStore;
use uuid::Uuid;

pub struct TestSetup {
    pub file_infos: Arc<DashMap<String, FileInfo>>,
    pub hash_infos: Arc<HashInfo>,
    pub _watcher: Box<dyn std::any::Any + Send>,
    bucket_name: String,
}

impl TestSetup {
    // Cleanup function to delete the KV bucket
    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Connect to NATS and delete the bucket
        let client = async_nats::connect("nats://localhost:4222").await
            .map_err(|_| "Failed to connect to NATS for cleanup")?;
        let jetstream = async_nats::jetstream::new(client);

        if let Err(e) = jetstream.delete_key_value(&self.bucket_name).await {
            eprintln!("Warning: Failed to delete KV bucket {}: {}", self.bucket_name, e);
        }
        Ok(())
    }
}

// Create a test KvStore for testing with unique bucket name
async fn create_test_kv_store() -> Result<(KvStore, String), Box<dyn std::error::Error>> {
    let client = async_nats::connect("nats://localhost:4222").await
        .map_err(|_| "Failed to connect to NATS for testing")?;
    let jetstream = async_nats::jetstream::new(client);

    // Use unique bucket name per test to avoid conflicts
    let unique_bucket = format!("test_hashes_{}", Uuid::new_v4().simple());

    let kv = jetstream.create_key_value(async_nats::jetstream::kv::Config {
        bucket: unique_bucket.clone(),
        ..Default::default()
    }).await
        .map_err(|e| format!("Failed to create KV store: {}", e))?;
    Ok((kv, unique_bucket))
}

pub async fn setup_test_environment() -> Result<TestSetup, Box<dyn std::error::Error>> {
    let watch_path = Path::new(".");
    let _  = set_watch_path(watch_path.to_path_buf());
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(Path::new("./fs_ignore"));

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());

    // Create unique KvStore for this test
    let (kv_store, bucket_name) = create_test_kv_store().await?;
    let hash_infos = Arc::new(HashInfo::new(kv_store));

    let _watcher = setup_debounced_watcher(
        watch_path,
        Arc::clone(&file_infos),
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;

    Ok(TestSetup {
        file_infos,
        hash_infos,
        _watcher: Box::new(_watcher),
        bucket_name,
    })
}

pub async fn wait_for_file_state(
    file_infos: &Arc<DashMap<String, FileInfo>>,
    path: &str,
    expected_state: Option<FileState>,
    timeout_ms: u64
) -> bool {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    while start.elapsed() < timeout {
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
            if let Some(info) = file_infos.get(path) {
                eprintln!("Found file state: {:?} (expecting {:?})", info.state, expected_state);
                if Some(info.state.clone()) == expected_state {
                    return true;
                }
            }
        }
        eprintln!("Sleeping");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    if expected_state.is_none() {
        eprintln!("Timed out waiting for file to be removed from map");
    } else {
        eprintln!("Timed out waiting for state: {:?} for file: {:?}", expected_state, path);
    }

    // eprintln!("File infos struct: {:?}", file_infos);

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
#[allow(dead_code)]
pub fn create_test_directory_hierarchy(base_path: &Path) -> Result<(String, Vec<String>), Box<dyn std::error::Error>> {
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
