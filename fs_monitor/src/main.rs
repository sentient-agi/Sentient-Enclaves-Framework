use notify::{recommended_watcher, Event, RecursiveMode, Result, Watcher, EventKind};
use notify::event::{ModifyKind, DataChange, CreateKind, AccessKind, AccessMode, RenameMode};
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
mod fs_ignore;
use fs_ignore::IgnoreList;

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    
    // Clone for the closure
    let file_infos_clone = Arc::clone(&file_infos);
    
    // Initialize the watcher
    let mut watcher = recommended_watcher(move |res: Result<Event>| {
        tx.send(res).expect("Failed to send event");
    })?;

    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;
    println!("Started watching current directory for changes...");
    
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list("/home/ec2-user/pipeline-tee.rs/fs_monitor/fs_ignore");
    
    // Start a thread to handle events
    thread::spawn(move || {
        for res in rx {
            match res {
                Ok(event) => {
                handle_event(event, &file_infos_clone, &ignore_list).unwrap_or_else(|e| {
                    eprintln!("Error handling event: {}", e);
                });
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
});

loop {
    println!("Enter path relative to current working directory to get hash of file");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let path = input.trim();
    let path = handle_path(path);
    println!("path: {}", path);
    let hash = retrieve_hash(&path, &file_infos)?;
    println!("Hash of {}: {}", path, hash);
}
}

fn retrieve_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>) -> Result<String> {
    // if path is directory, return all hashes in directory
    if fs::metadata(path)?.is_dir() {
        eprintln!("path is directory: {}", path);
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

fn handle_event(event: Event, file_infos: &Arc<DashMap<String, FileInfo>>, ignore_list: &IgnoreList) -> Result<()> {
    let paths_old: Vec<String> = event.paths.iter()
        .filter_map(|p| p.to_str().map(|s| s.to_string()))
        .collect();

    let mut paths = Vec::new();
    for path in paths_old {
        let path = handle_path(&path);
        paths.push(path);
    }
    if paths.is_empty() {
        return Ok(());
    }
    if paths.len() == 1 {
        // eprintln!("Single path in event: {:?} {:?}", event.kind, paths);
        let path = paths[0].clone();
        // Ignore this event if it matches the regex pattern specified in fs_ignore
        if ignore_list.is_ignored(&path) {
            // eprintln!("Ignoring event {:?} for path: {}", event.kind, path);
            return Ok(());
        }
    }

    if paths.len() > 1 {

        // If all paths in event are ignored, skip the event
        if paths.iter().all(|path| ignore_list.is_ignored(path)) {
            return Ok(());
        }

    }

    let infos = file_infos.clone();    

    match event.kind {
        EventKind::Create(CreateKind::File) => {
            handle_file_creation(paths.clone(), &file_infos);
        }


        EventKind::Modify(ModifyKind::Data(DataChange::Any) ) => {
           handle_file_data_modification(paths.clone(), &file_infos); 
        }
        // Need to handle Rename, Delete, etc.
        EventKind::Modify(ModifyKind::Name(rename_mode)) => {
            match rename_mode {
                RenameMode::Both => {
                    handle_file_rename(paths.clone(), &file_infos, &ignore_list);
                },
                _ => {
                    // eprintln!("Unhandled rename mode: {:?} for paths: {:?}", rename_mode, paths);
                }
            }
        },
        // Handling end of file write
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
            handle_file_save_on_write(paths.clone(), &file_infos);
        }
        _ => {
            for path in paths {
                eprintln!("#Unhandled event {:?} for: {}", event.kind, path);
            }
        }
    }

    Ok(())
}

fn handle_file_creation(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    if paths.len() != 1 {
        eprintln!("Create event has multiple paths: {:?}", paths);
        return;
    }
    let path = paths[0].clone();
    eprintln!("File created: {}", path);
    file_infos.insert(path.clone(), FileInfo {
            file_type: FileType::File,
            state: FileState::Created,
            hash_info: None,
            version: 0,
    });
}

fn handle_file_data_modification(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    if paths.len() != 1 {
        eprintln!("Modify event has multiple paths: {:?}", paths);
        return;
    }
    let path = paths[0].clone();
    // eprintln!("File modified: {}", path);
    if let Some(mut file_info) = file_infos.get_mut(&path) {
        if file_info.file_type == FileType::File {
            file_info.state = FileState::Modified;
            file_info.hash_info = None; // Reset hash since file is modified
        }
    }
}

fn handle_file_save_on_write(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    if paths.len() != 1 {
        eprintln!("Save on write event has multiple paths: {:?}", paths);
        return;
    }
    let path = paths[0].clone();
    if let Some(mut file_info) = file_infos.get_mut(&path) {
        if file_info.file_type == FileType::File && file_info.state == FileState::Modified {
            eprintln!("File closed after write: {}", path);

            file_info.version += 1;
            eprintln!("File {} is ready for hashing.", path);
            file_info.state = FileState::Closed;
            perform_file_hashing(path.clone(), &file_infos);
        }
    }
}


// TODO: Handle rename events
fn handle_file_rename(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, ignore_list: &IgnoreList) {
    if paths.len() != 2 {
        eprintln!("Rename event should have 2 paths: {:?}", paths);
        return;
    }
    let from_path = paths[0].clone();
    let to_path = paths[1].clone();
    if ignore_list.is_ignored(&to_path) {
        // This means the already monitored file is being renamed to something that is ignored
        // We can ignore this event right now but ideally this should remove the entry from the file_infos
        // We can ignore this event right now but ideally this should remove the entry from the file_infos
        eprintln!("Ignoring rename event for path: {}", to_path);
        return;
    }
    else if ignore_list.is_ignored(&from_path) {
        // This marks that an ignored file is being renamed to something that should be monitored.
        // This won't usually trigger create or modify events for smaller files.
        // We need to create a new entry in file_infos for the new path if it doesn't exist
        // and then add the hash of the file to the new path
        eprintln!("Handling rename event for paths: {} -> {}", from_path, to_path);
        if !file_infos.contains_key(&to_path) {
            file_infos.insert(to_path.clone(), FileInfo {
                file_type: FileType::File,
                state: FileState::Renamed,
                hash_info: None,
                version: 0,
            });
            perform_file_hashing(to_path.clone(), &file_infos);

        }

        return;
    }
    else {
        // This is a normal rename event where the file is being renamed to some other name.
        // We need to update the entry in file_infos for the new path. The hash of the file should be recalculated?
        // Currently this event is also ignored.
        eprintln!("File renamed from {} to {}", from_path, to_path);
       
    }
}

fn perform_file_hashing(path: String, file_infos: &Arc<DashMap<String, FileInfo>>) {
    // eprintln!("path: {}", path);
    // let file_info = Arc::clone(&file_infos);
    let file_infos = Arc::clone(&file_infos);
    thread::spawn(move || -> Result<String> {
        file_infos.get_mut(&path).unwrap().hash_info = Some(HashInfo {
            hash_state: HashState::InProgress,
            hash_string: None,
        });
        match calculate_hash(&path) {
            Ok(hash) => {
                file_infos.get_mut(&path).unwrap().hash_info = Some(HashInfo {
                    hash_state: HashState::Complete,
                    hash_string: Some(hash.clone()),
                });
                eprintln!("Hash calculated for {}: {}", path, hash);
                Ok(hash)
            }
            Err(e) => {
                file_infos.get_mut(&path).unwrap().hash_info = Some(HashInfo {
                    hash_state: HashState::Error,
                        hash_string: None,
                });
                eprintln!("Error calculating hash for {}: {}", path, e);
                Ok("Failed to calculate hash".to_string())
            }
        }
    });
}

fn calculate_hash(path: &str) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash_result = hasher.finalize();
    Ok(format!("{:x}", hash_result))
}
fn handle_path(path: &str) -> String {
    // check if path is absolute
    // if it is then make it relative
    // with respect to the current working directory
    if path.starts_with("/") {
        let current_dir = std::env::current_dir().unwrap();
        let relative_path = Path::new(&path).strip_prefix(current_dir).unwrap();
        // // if there is ./ in the path then remove it
        if relative_path.starts_with("./") {
            let relative_path = relative_path.strip_prefix("./").unwrap();
        }
        // update the path with the relative path
        let path = relative_path.to_str().unwrap();
        // eprintln!("Relative path: {}", path);
        path.to_string()
    }
    else {
        path.to_string()
    }    
}