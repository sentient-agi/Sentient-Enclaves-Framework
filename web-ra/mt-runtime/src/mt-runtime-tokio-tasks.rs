use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io;
use tokio::sync::Mutex;
use sha3::{Digest, Sha3_512};
use async_recursion::async_recursion;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> io::Result<()> {
    let dir_path = PathBuf::from("./tmp_dir");
    let results = recursive_hash_dir(dir_path).await?;

    println!("\nFinal Hash Results:");
    for (file_path, hash) in results {
        println!("{}: {}", file_path, hex::encode(hash));
    }
    Ok(())
}

async fn recursive_hash_dir(dir_path: PathBuf) -> io::Result<HashMap<String, Vec<u8>>> {
    let results = Arc::new(Mutex::new(HashMap::new()));
    let tasks = Arc::new(Mutex::new(HashMap::new()));

    // Recursively visit files and spawn tasks
    visit_files_recursively(dir_path, tasks.clone()).await?;

    // Process all tasks
    let mut tasks_lock = tasks.lock().await;
    while !tasks_lock.is_empty() {
        // Use keys to avoid borrowing issues
        let keys: Vec<String> = tasks_lock.keys().cloned().collect();
        for file_path in keys {
            if let Some(task) = tasks_lock.remove(&file_path) {
                let results = results.clone();
                tokio::spawn(async move {
                    match task.await {
                        Ok(Ok(hash)) => results.lock().await.insert(file_path, hash),
                        Ok(Err(e)) => {
                            eprintln!("Error processing {:?}: {:?}", file_path, e);
                            Some(Vec::new())
                        },
                        Err(e) => {
                            eprintln!("Task panicked for {:?}: {:?}", file_path, e);
                            Some(Vec::new())
                        },
                    }
                });
            }
        }

        // Allow other tasks to progress
        tokio::task::yield_now().await;
    }

    Arc::try_unwrap(results)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to unwrap results"))
        .map(|m| m.into_inner())
}

#[async_recursion]
async fn visit_files_recursively(
    path: PathBuf,
    tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<io::Result<Vec<u8>>>>>>,
) -> io::Result<()> {
    let metadata = tokio::fs::metadata(&path).await?;

    if metadata.is_dir() {
        let mut entries = tokio::fs::read_dir(&path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            visit_files_recursively(path, tasks.clone()).await?;
        }
    } else if metadata.is_file() {
        let file_path_hash = path.to_string_lossy().to_string();
        let file_path_task = file_path_hash.clone();
        let task = tokio::task::spawn_blocking(move || hash_file(file_path_hash.as_str()));
        tasks.lock().await.insert(file_path_task, task);
    }

    Ok(())
}

fn hash_file(file_path: &str) -> std::io::Result<Vec<u8>> {
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

    Ok(hasher.finalize().to_vec())
}
