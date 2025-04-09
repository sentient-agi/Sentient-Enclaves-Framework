use notify::{Event, Result, EventKind};
use notify::event::{ModifyKind, DataChange, CreateKind, AccessKind, AccessMode, RenameMode};
use std::sync::Arc;
use dashmap::DashMap;
use crate::hash::storage::{HashInfo, perform_file_hashing, remove_stale_tasks};
use crate::fs_ops::state::{FileInfo, FileState, FileType};
use crate::fs_ops::ignore::IgnoreList;
use crate::fs_ops::path_utils::handle_path;

pub fn handle_event(event: Event, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>, ignore_list: &IgnoreList) -> Result<()> {
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
        let path = paths[0].clone();
        if ignore_list.is_ignored(&path) {
            return Ok(());
        }
    }
    if paths.len() > 1 {
        // If all paths in event are ignored, skip the event
        if paths.iter().all(|path| ignore_list.is_ignored(path)) {
            return Ok(());
        }
    }

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
                    handle_file_rename(paths.clone(), &file_infos, &hash_info, &ignore_list);
                },
                _ => {
                    // eprintln!("Unhandled rename mode: {:?} for paths: {:?}", rename_mode, paths);
                }
            }
        },
        // Handling end of file write
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
            handle_file_save_on_write(paths.clone(), &file_infos, &hash_info);
        }
        _ => {
        }
    }

    Ok(())
}

pub fn handle_file_creation(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    if paths.len() != 1 {
        eprintln!("Create event has multiple paths: {:?}", paths);
        return;
    }
    let path = paths[0].clone();
    eprintln!("File created: {}", path);
    file_infos.insert(path.clone(), FileInfo {
            file_type: FileType::File,
            state: FileState::Created,
            version: 0,
    });
}

pub fn handle_file_data_modification(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    if paths.len() != 1 {
        eprintln!("Modify event has multiple paths: {:?}", paths);
        return;
    }
    let path = paths[0].clone();
    
    if let Some(mut file_info) = file_infos.get_mut(&path) {
        if file_info.file_type == FileType::File {
            file_info.state = FileState::Modified;
            // No need to modify hash here as we keep versions of hashes
        }
    }
}

pub fn handle_file_save_on_write(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) {
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
            tokio::spawn(perform_file_hashing(path.clone(), Arc::clone(hash_info)));
        }
    }
}

pub fn handle_file_rename(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>, ignore_list: &IgnoreList) {
    if paths.len() != 2 {
        eprintln!("Rename event should have 2 paths: {:?}", paths);
        return;
    }
    let from_path = paths[0].clone();
    let to_path = paths[1].clone();
    if ignore_list.is_ignored(&to_path) {
        // This means the already monitored file is being renamed to something that is ignored
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
                // File can only be renamed when it's closed
                state: FileState::Closed,
                version: 1,
            });
            tokio::spawn(perform_file_hashing(to_path.clone(), Arc::clone(hash_info)));
        }

        return;
    }
    else {
        // This is a normal rename event where the file is being renamed to some other name.
        eprintln!("File renamed from {} to {}", from_path, to_path);
        
        // Remove any on-going hashing tasks for old file
        
        // Copy the file_info state for new file
        if let Some(file_info) = file_infos.get(&from_path).map(|info| info.clone()) {
            let from_path_clone = from_path.clone();
            let to_path_clone = to_path.clone();
            let file_infos_clone = Arc::clone(file_infos);
            let hash_info_clone = Arc::clone(hash_info);
            
            tokio::spawn(async move {
                // Calculate hash for the new file
                match crate::hash::hasher::hash_file(&to_path_clone) {
                    Ok(latest_hash) => {
                        // Try to get the old hash for comparison
                        match crate::hash::storage::retrieve_file_hash(&from_path_clone, &file_infos_clone, &hash_info_clone).await {
                            Ok(old_hash) => {
                                if latest_hash != old_hash {
                                    eprintln!("Warning: Hash mismatch after rename from {} to {}", 
                                             from_path_clone, to_path_clone);
                                }
                                
                                // Update entry in the dashmap regardless
                                if let Some((_, info)) = file_infos_clone.remove(&from_path_clone) {
                                    file_infos_clone.insert(to_path_clone.clone(), info);
                                    
                                    // Transfer hash history if available
                                    if let Ok(mut results) = hash_info_clone.hash_results.try_lock() {
                                        if let Some(hashes) = results.remove(&from_path_clone) {
                                            results.insert(to_path_clone, hashes);
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                eprintln!("Error retrieving hash for {}: {}", from_path_clone, e);
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Error hashing file {}: {}", to_path_clone, e);
                    }
                }
            });
        }
    }
} 