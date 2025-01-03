use notify::{recommended_watcher, Event, RecursiveMode, Result, Watcher, EventKind};
use notify::event::{ModifyKind, DataChange, CreateKind, AccessKind, AccessMode};
use std::sync::mpsc;
use std::path::Path;
use std::fs;
use sha2::{Sha256, Digest};
use std::sync::Arc;
use std::thread;
use dashmap::DashMap;
// Import the FileState and FileInfo structs
mod state;
use state::{FileState, FileInfo, FileType, HashState, HashInfo};

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();
    // Use Arc and Mutex for thread-safe shared state
    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    
    // Clone for the closure
    let file_infos_clone = Arc::clone(&file_infos);
    
    // Initialize the watcher
    let mut watcher = recommended_watcher(move |res: Result<Event>| {
        tx.send(res).expect("Failed to send event");
    })?;

    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;
    println!("Started watching current directory for changes...");
    

    // Start a thread to handle events
    thread::spawn(move || {
        for res in rx {
            match res {
                Ok(event) => {
                handle_event(event, &file_infos_clone).unwrap_or_else(|e| {
                    eprintln!("Error handling event: {}", e);
                });
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
});

loop {
    println!("Enter absolute path to get hash of file");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let path = input.trim();

    let hash = retrieve_hash(path, &file_infos)?;
    println!("Hash of {}: {}", path, hash);
}
}

fn retrieve_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>) -> Result<String> {
    // if path is directory, return all hashes in directory
    if fs::metadata(path)?.is_dir() {
        let mut hashes = String::new();
        // Still need to iterate for directories
        for ref_multi in file_infos.iter() {
            if ref_multi.key().starts_with(path) {
                if let Some(hash) = &ref_multi.value().hash_info.as_ref().unwrap().hash_string {
                    hashes.push_str(&format!("{}: {}\n", ref_multi.key(), hash));
                }
            }
        }
        return Ok(hashes);
    }

    // For single files, use direct lookup instead of iteration
    match file_infos.get(path) {
        Some(info) => match &info.hash_info.as_ref().unwrap().hash_string {
            Some(hash) => Ok(hash.clone()),
            None => Ok(format!("File found but no hash available: {}", path))
        },
        None => Ok(format!("File not found: {}", path))
    }
}

fn handle_event(event: Event, file_infos: &Arc<DashMap<String, FileInfo>>) -> Result<()> {
    let paths: Vec<String> = event.paths.iter()
        .filter_map(|p| p.to_str().map(|s| s.to_string()))
        .collect();

    let mut infos = file_infos;

    match event.kind {
        EventKind::Create(kind) => {
            for path in paths {
                if let CreateKind::File = kind {
                    eprintln!("File created: {}", path);
                    infos.insert(path.clone(), FileInfo {
                        file_type: FileType::File,
                        state: FileState::Created,
                        hash_info: None,
                    });
                }
            }
        }

        EventKind::Modify(modify_kind) => {
            for path in paths {
                match modify_kind {
                    ModifyKind::Data(DataChange::Any) => {
                        // println!("File modified: {}", path);
                        if let Some(mut file_info) = infos.get_mut(&path) {
                            if file_info.file_type == FileType::File {
                                file_info.state = FileState::Modified;
                                file_info.hash_info = None; // Reset hash since file is modified
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // Need to handle Rename, Delete, etc.

        // Handling end of file write
        EventKind::Access(access_kind) => {
            if let AccessKind::Close(AccessMode::Write) = access_kind {
                // This marks the file has been written to and is now closed.
                for path in paths {
                    // Skip files in .cache directory
                    if !path.contains("/.cache/") {
                        if let Some(mut file_info) = infos.get_mut(&path) {
                            if file_info.file_type == FileType::File && file_info.state == FileState::Modified {
                                eprintln!("File closed after write: {}", path);

                            // Make the file immutable
                            make_file_immutable(&path)?;
                            eprintln!("File {} is now immutable.", path);
                            file_info.state = FileState::Immutable;
                            
                            file_info.hash_info = Some(HashInfo {
                                hash_state: HashState::InProgress,
                                hash_string: None,
                            });

                            // Calculate hash using a new thread
                            let path_clone = path.clone();
                            let infos_clone = Arc::clone(&infos);
                            thread::spawn(move || -> Result<String> {
                                match calculate_hash(&path_clone) {
                                    Ok(hash) => {
                                       if let Some(mut file_info) = infos_clone.get_mut(&path_clone) {
                                            file_info.state = FileState::Immutable;
                                            file_info.hash_info = Some(HashInfo {
                                                hash_state: HashState::Complete,
                                                hash_string: Some(hash.clone()),
                                            });
                                        }
                                        eprintln!("Hash calculated for {}: {}", path_clone, hash);
                                        Ok(hash)
                                    }
                                    Err(e) => {
                                        if let Some(mut file_info) = infos_clone.get_mut(&path_clone) {
                                            file_info.state = FileState::Immutable;
                                            file_info.hash_info = Some(HashInfo {
                                                hash_state: HashState::Error,
                                                hash_string: None,
                                            });
                                        }
                                        eprintln!("Error calculating hash for {}: {}", path_clone, e);
                                        Ok("Failed to calculate hash".to_string())
                                    }
                                }
                            });
                        }
                    }
                }
                }
            }
        }
        _ => {
            for path in paths {
                eprintln!("Unhandled event {:?} for: {}", event.kind, path);
            }
        }
    }

    Ok(())
}


fn calculate_hash(path: &str) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}

fn make_file_immutable(path: &str) -> Result<()> {
    // Implementation depends on the operating system.
    // For Unix-based systems, remove write permissions.
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        // Remove write permissions for owner, group, and others
        perms.set_mode(perms.mode() & !(0o222));
        fs::set_permissions(path, perms)?;
    Ok(())
}