use notify::{recommended_watcher, Event, RecursiveMode, Result, Watcher, EventKind};
use notify::event::{ModifyKind, DataChange, CreateKind, AccessKind, AccessMode};
use std::sync::mpsc;
use std::path::Path;
use std::collections::HashMap;
use std::fs;
use sha2::{Sha256, Digest};
use std::sync::{Arc, Mutex};
use std::thread;

// Import the FileState and FileInfo structs
mod state;
use state::{FileState, FileInfo, FileType};

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();
    // Use Arc and Mutex for thread-safe shared state
    let file_infos: Arc<Mutex<HashMap<String, FileInfo>>> = Arc::new(Mutex::new(HashMap::new()));
    
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

fn retrieve_hash(path: &str, file_infos: &Arc<Mutex<HashMap<String, FileInfo>>>) -> Result<String> {
    let infos = file_infos.lock().unwrap();
    
    // if path is directory, return all hashes in directory
    let mut hashes = String::new();
    if fs::metadata(path)?.is_dir() {
        for (key, value) in infos.iter() {
            if key.contains(path) {
                eprintln!("Hash of {}: {}", key, value.hash.as_ref().unwrap());
                hashes.push_str(&value.hash.as_ref().unwrap().clone());
                hashes.push_str("\n");
            }
        }
        return Ok(hashes);
    }

    // if path is file, return hash of file
    else {
        for (key, value) in infos.iter() {
            if key == path {
                return Ok(value.hash.as_ref().unwrap().clone());
            }
        }
    }
    Ok(format!("File not found: {}", path))
}

fn handle_event(event: Event, file_infos: &Arc<Mutex<HashMap<String, FileInfo>>>) -> Result<()> {
    let paths: Vec<String> = event.paths.iter()
        .filter_map(|p| p.to_str().map(|s| s.to_string()))
        .collect();

    let mut infos = file_infos.lock().unwrap();

    match event.kind {
        EventKind::Create(kind) => {
            for path in paths {
                if let CreateKind::File = kind {
                    eprintln!("File created: {}", path);
                    infos.insert(path.clone(), FileInfo {
                        file_type: FileType::File,
                        state: FileState::Created,
                        hash: None,
                    });
                }
            }
        }

        EventKind::Modify(modify_kind) => {
            for path in paths {
                match modify_kind {
                    ModifyKind::Data(DataChange::Any) => {
                        // println!("File modified: {}", path);
                        if let Some(file_info) = infos.get_mut(&path) {
                            if file_info.file_type == FileType::File {
                                file_info.state = FileState::Modified;
                                file_info.hash = None; // Reset hash since file is modified
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
                // This marks the file has been written to.
                for path in paths {
                    // Skip files in .cache directory
                    if !path.contains("/.cache/") {
                        if let Some(file_info) = infos.get_mut(&path) {
                            if file_info.file_type == FileType::File && file_info.state == FileState::Modified {
                                eprintln!("File closed after write: {}", path);

                            // Make the file immutable (implementation dependent)
                            make_file_immutable(&path)?;
                            eprintln!("File {} is now immutable.", path);
                            // Calculate hash
                            let hash = calculate_hash(&path)?;
                            eprintln!("Hash calculated for {}: {}", path, hash);
                            // Update state to Immutable and store hash
                            file_info.state = FileState::Immutable;
                            file_info.hash = Some(hash.clone());
                            
                            
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