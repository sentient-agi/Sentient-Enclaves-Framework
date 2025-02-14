use async_std::fs::{self, File};
use async_std::io::{self, BufWriter};
use async_std::path::Path;
use async_std::prelude::*;
use async_std::task::{Context, Poll};
use futures::task::noop_waker;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::pin::Pin;

#[async_std::main]
async fn main() -> io::Result<()> {
    let dir_path = "example_directory";
    let mut results = recursive_hash_dir(dir_path).await?;

    println!("\nFinal Hash Results:");
    for (file_path, hash) in results.drain() {
        println!("{}: {:x}", file_path, hash);
    }

    Ok(())
}

/// Recursively hashes all files in the given directory and tracks readiness of hashing futures.
async fn recursive_hash_dir(dir_path: &str) -> io::Result<HashMap<String, Vec<u8>>> {
    let mut results = HashMap::new();
    let mut tasks = Vec::new();

    // Visit each file recursively and spawn hashing tasks
    visit_files_recursively(Path::new(dir_path), &mut tasks).await?;

    // Track readiness of tasks in a cycle
    let mut pinned_tasks: Vec<_> = tasks
        .into_iter()
        .map(|(file_path, future)| (file_path, Box::pin(future)))
        .collect();

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    while !pinned_tasks.is_empty() {
        let mut remaining_tasks = Vec::new();
        for (file_path, mut task) in pinned_tasks {
            match task.as_mut().poll(&mut cx) {
                Poll::Ready(Ok(hash)) => {
                    results.insert(file_path, hash);
                }
                Poll::Ready(Err(e)) => {
                    eprintln!("Error processing {}: {}", file_path, e);
                }
                Poll::Pending => {
                    remaining_tasks.push((file_path, task));
                }
            }
        }
        pinned_tasks = remaining_tasks;

        // Yield to let other async tasks progress
        async_std::task::yield_now().await;
    }

    Ok(results)
}

/// Visits all files recursively in a directory and spawns hashing tasks.
async fn visit_files_recursively(
    path: &Path,
    tasks: &mut Vec<(String, impl Future<Output = io::Result<Vec<u8>>>)>,
) -> io::Result<()> {
    if path.is_dir().await {
        let mut entries = fs::read_dir(path).await?;
        while let Some(entry) = entries.next().await {
            let entry = entry?;
            visit_files_recursively(entry.path().as_path(), tasks).await?;
        }
    } else if path.is_file().await {
        let file_path = path.to_string_lossy().to_string();
        let task = async move { hash_file(&file_path).await };
        tasks.push((file_path, task));
    }
    Ok(())
}

/// Hashes a single file using SHA-512.
async fn hash_file(file_path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(file_path).await?;
    let mut hasher = Sha512::new();

    // Create a writer adapter for the hasher
    let mut hasher_writer = BufWriter::new(HasherWriter::new(&mut hasher));

    // Copy file contents into the hasher
    io::copy(&mut file, &mut hasher_writer).await?;

    // Finalize the hash
    Ok(hasher.finalize().to_vec())
}

/// A wrapper around a hasher to implement `io::Write`.
struct HasherWriter<'a, H: Digest> {
    hasher: &'a mut H,
}

impl<'a, H: Digest> HasherWriter<'a, H> {
    fn new(hasher: &'a mut H) -> Self {
        Self { hasher }
    }
}

impl<H: Digest> io::Write for HasherWriter<'_, H> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
