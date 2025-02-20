use async_std::fs::{self, File};
use async_std::io::{self, BufWriter};
use async_std::path::Path;
use async_std::prelude::*;
use async_std::task::{Context, Poll, spawn_blocking};
use futures::task::noop_waker;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::pin::Pin;

#[async_std::main]
async fn main() -> io::Result<()> {
    let dir_path = "example_directory";

    let results = recursive_hash_dir(dir_path).await?;

    println!("\nFinal Hash Results:");
    for (file_path, hash) in results {
        println!("{}: {:x}", file_path, hash);
    }

    Ok(())
}

/// Recursively hashes all files in the given directory, using a `HashMap` for task tracking.
async fn recursive_hash_dir(dir_path: &str) -> io::Result<HashMap<String, Vec<u8>>> {
    let mut results = HashMap::new();
    let mut tasks: HashMap<String, Pin<Box<async_std::task::JoinHandle<io::Result<Vec<u8>>>>>> = HashMap::new();

    // Visit all files recursively and spawn hashing tasks
    visit_files_recursively(Path::new(dir_path), &mut tasks).await?;

    // Check readiness of tasks
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    while !tasks.is_empty() {
        check_task_readiness(&mut tasks, &mut results, &mut cx).await;
    }

    Ok(results)
}

/// Checks readiness of tasks and processes ready ones.
async fn check_task_readiness(
    tasks: &mut HashMap<String, Pin<Box<async_std::task::JoinHandle<io::Result<Vec<u8>>>>>>,
    results: &mut HashMap<String, Vec<u8>>,
    cx: &mut Context<'_>,
) {
    let mut completed_tasks = Vec::new();

    for (file_path, task) in tasks.iter_mut() {
        match task.as_mut().poll(cx) {
            Poll::Ready(Ok(Ok(hash))) => {
                results.insert(file_path.clone(), hash);
                completed_tasks.push(file_path.clone());
            }
            Poll::Ready(Ok(Err(e))) => {
                eprintln!("Error processing {}: {}", file_path, e);
                completed_tasks.push(file_path.clone());
            }
            Poll::Ready(Err(e)) => {
                eprintln!("Task panicked for {}: {}", file_path, e);
                completed_tasks.push(file_path.clone());
            }
            Poll::Pending => {}
        }
    }

    // Remove completed tasks from the hashmap
    for file_path in completed_tasks {
        tasks.remove(&file_path);
    }

    // Yield to allow other async tasks to make progress
    async_std::task::yield_now().await;
}

/// Visits all files recursively in a directory and spawns hashing tasks.
async fn visit_files_recursively(
    path: &Path,
    tasks: &mut HashMap<String, Pin<Box<async_std::task::JoinHandle<io::Result<Vec<u8>>>>>>,
) -> io::Result<()> {
    if path.is_dir().await {
        let mut entries = fs::read_dir(path).await?;
        while let Some(entry) = entries.next().await {
            let entry = entry?;
            visit_files_recursively(entry.path().as_path(), tasks).await?;
        }
    } else if path.is_file().await {
        let file_path = path.to_string_lossy().to_string();
        let task = spawn_blocking(move || {
            // Perform hashing in a separate thread
            hash_file(&file_path)
        });
        tasks.insert(file_path, Box::pin(task));
    }
    Ok(())
}

/// Hashes a single file using SHA-512.
fn hash_file(file_path: &str) -> io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(file_path)?;
    let mut hasher = Sha512::new();
    let mut buffer = [0; 8192];

    // Read the file in chunks and update the hash
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    // Finalize and return the hash
    Ok(hasher.finalize().to_vec())
}
