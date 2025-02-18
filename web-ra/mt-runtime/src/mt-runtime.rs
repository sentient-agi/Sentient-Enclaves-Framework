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

/// Recursively hashes all files in the given directory, tracking task readiness.
async fn recursive_hash_dir(dir_path: &str) -> io::Result<HashMap<String, Vec<u8>>> {
    let mut results = HashMap::new();
    let mut tasks = Vec::new();

    // Visit all files recursively and spawn hashing tasks
    visit_files_recursively(Path::new(dir_path), &mut tasks).await?;

    // Pin the tasks for readiness polling
    let mut pinned_tasks: Vec<_> = tasks
        .into_iter()
        .map(|task| Box::pin(task))
        .collect();

    // Check readiness of tasks
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    while !pinned_tasks.is_empty() {
        pinned_tasks = check_task_readiness(&mut pinned_tasks, &mut results, &mut cx).await;
    }

    Ok(results)
}

/// Checks readiness of tasks and processes ready ones.
async fn check_task_readiness(
    tasks: &mut Vec<Pin<Box<async_std::task::JoinHandle<io::Result<(String, Vec<u8>)>>>>>,
    results: &mut HashMap<String, Vec<u8>>,
    cx: &mut Context<'_>,
) -> Vec<Pin<Box<async_std::task::JoinHandle<io::Result<(String, Vec<u8>)>>>>> {
    let mut remaining_tasks = Vec::new();

    for mut task in tasks.drain(..) {
        match task.as_mut().poll(cx) {
            Poll::Ready(Ok(Ok((file_path, hash)))) => {
                results.insert(file_path, hash);
            }
            Poll::Ready(Ok(Err(e))) => {
                eprintln!("Error: {}", e);
            }
            Poll::Ready(Err(e)) => {
                eprintln!("Task panicked: {}", e);
            }
            Poll::Pending => {
                remaining_tasks.push(task);
            }
        }
    }

    // Yield to allow other async tasks to make progress
    async_std::task::yield_now().await;

    remaining_tasks
}

/// Visits all files recursively in a directory and spawns hashing tasks.
async fn visit_files_recursively(
    path: &Path,
    tasks: &mut Vec<async_std::task::JoinHandle<io::Result<(String, Vec<u8>)>>>,
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
            let result = hash_file(&file_path);
            result.map(|hash| (file_path, hash))
        });
        tasks.push(task);
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
