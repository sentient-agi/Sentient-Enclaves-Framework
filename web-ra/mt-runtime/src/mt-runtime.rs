// use async_std::fs::{self, File};
// use async_std::io::{self, BufWriter, Read};
// use std::fs::{self, File};
// use std::io::{self, BufWriter, Read};
use async_std::io as async_io;
use async_std::fs as async_fs;
use std::io::Read;
use async_std::path::Path;
use async_std::prelude::*;
use async_std::task::{Context, Poll, spawn_blocking};
use futures::task::noop_waker;
use sha3::{Digest, Sha3_512};
use std::collections::HashMap;
use std::pin::Pin;
// use futures::FutureExt;
// use futures::future::{BoxFuture, FutureExt};
use async_recursion::async_recursion;

#[tokio::main]
async fn main() -> async_io::Result<()> {
    let dir_path = "./tmp_dir";

    let results = recursive_hash_dir(dir_path).await?;

    println!("\nFinal Hash Results:");
    for (file_path, hash) in results {
        println!("{}: {}", file_path, hex::encode(hash));
    }

    Ok(())
}

/// Recursively hashes all files in the given directory, using a `HashMap` for task tracking.
async fn recursive_hash_dir(dir_path: &str) -> async_io::Result<HashMap<String, Vec<u8>>> {
    let mut results = HashMap::new();
    let mut tasks: HashMap<String, Pin<Box<async_std::task::JoinHandle<std::io::Result<Vec<u8>>>>>> = HashMap::new();

    // Visit all files recursively and spawn hashing tasks
    visit_files_recursively(Path::new(dir_path), &mut tasks).await?;

    // Check readiness of tasks
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    while !tasks.is_empty() {
        let mut completed_tasks = Vec::new();

        for (file_path, task) in tasks.iter_mut() {
            if check_task_readiness(file_path, task, &mut results, &mut cx).await {
                completed_tasks.push(file_path.clone());
            }
        }

        // Remove completed tasks from the hashmap
        for file_path in completed_tasks {
            tasks.remove(&file_path);
        }

        // Yield to allow other async tasks to make progress
        async_std::task::yield_now().await;
    }

    Ok(results)
}

/// Checks the readiness of a specific task by file path.
async fn check_task_readiness(
    file_path: &str,
    task: &mut Pin<Box<async_std::task::JoinHandle<std::io::Result<Vec<u8>>>>>,
    results: &mut HashMap<String, Vec<u8>>,
    cx: &mut Context<'_>,
) -> bool {
    match task.as_mut().poll(cx) {
        Poll::Ready(Ok(hash)) => {
            results.insert(file_path.to_string(), hash);
            true // Task is complete
        }
        Poll::Ready(Err(e)) => {
            eprintln!("Error processing {}: {}", file_path, e);
            true // Task is complete
        }
//        Poll::Ready(Err(e)) => {
//            eprintln!("Task panicked for {}: {}", file_path, e);
//            true // Task is complete
//        }
        Poll::Pending => false, // Task is not complete
    }
}

/// Visits all files recursively in a directory and spawns hashing tasks.
#[async_recursion(Sync)]
async fn visit_files_recursively<'a>(
    path: &'a Path,
    tasks: &'a mut HashMap<String, Pin<Box<async_std::task::JoinHandle<std::io::Result<Vec<u8>>>>>>,
) -> async_io::Result<()>
{
    if path.is_dir().await {
        let mut entries = async_fs::read_dir(path).await?;
        while let Some(entry) = entries.next().await {
            let entry = entry?;
            visit_files_recursively(entry.path().as_path(), tasks).await?;
        }
    } else if path.is_file().await {
        let file_path_hash = path.to_string_lossy().to_string();
        let file_path_task = file_path_hash.clone();
        let task = spawn_blocking(move || {
            // Perform hashing in a separate thread
            hash_file(&file_path_hash)
        });
        tasks.insert(file_path_task, Box::pin(task));
    }
    Ok(())
}

/// Hashes a single file using SHA3-512.
fn hash_file(file_path: &str) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(file_path)?;
    let mut hasher = Sha3_512::new();
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
