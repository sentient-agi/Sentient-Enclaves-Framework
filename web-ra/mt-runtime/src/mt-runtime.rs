use async_std::fs::{self, File};
use async_std::io::{self, BufWriter};
use async_std::path::Path;
use async_std::prelude::*;
use async_std::task::spawn_blocking;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::sync::Arc;

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

/// Recursively hashes all files in the given directory, running each task in a separate thread.
async fn recursive_hash_dir(dir_path: &str) -> io::Result<HashMap<String, Vec<u8>>> {
    let mut results = HashMap::new();
    let mut tasks = Vec::new();

    // Visit all files recursively and spawn hashing tasks in separate threads
    visit_files_recursively(Path::new(dir_path), &mut tasks).await?;

    // Await all hashing tasks
    for task in tasks {
        match task.await {
            Ok((file_path, hash)) => {
                results.insert(file_path, hash);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(results)
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
