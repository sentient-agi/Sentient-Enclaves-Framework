use notify::{Event, Result, EventKind};
use notify::event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode};
use std::sync::Arc;
use dashmap::DashMap;
use crate::hash::storage::{HashInfo, perform_file_hashing, hash_cleanup};
use crate::fs_ops::state::{FileInfo, FileState, FileType};
use crate::fs_ops::ignore::IgnoreList;
use crate::fs_ops::fs_utils::handle_path;

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
        EventKind::Create(CreateKind::Folder) => {
            handle_directory_creation(paths.clone(), &file_infos);
        }
        EventKind::Modify(ModifyKind::Data(DataChange::Any) ) => {
           handle_file_data_modification(paths.clone(), &file_infos); 
        }
        EventKind::Remove(RemoveKind::File) => {
            handle_file_deletion(paths.clone(), &file_infos, &hash_info);
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
            // eprintln!("Unhandled event kind: {:?} for paths: {:?} ",event.kind, paths);
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
            version: 0,
    });
}

fn handle_directory_creation(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    if paths.len() != 1 {
        eprintln!("Create directory event has multiple paths: {:?}", paths);
        return;
    }
    let path = paths[0].clone();
    eprintln!("Directory created: {}", path);
    file_infos.insert(path.clone(), FileInfo {
            file_type: FileType::Directory,
            state: FileState::Closed, // Directories don't have the same lifecycle as files
            version: 0,
    });
}

fn handle_file_data_modification(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
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

fn handle_file_save_on_write(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) {
    if paths.len() != 1 {
        eprintln!("Save on write event has multiple paths: {:?}", paths);
        return;
    }
    let path = paths[0].clone();
    if let Some(mut file_info) = file_infos.get_mut(&path) {
        if file_info.file_type == FileType::File && file_info.state == FileState::Modified {
            // Calculate hash first and then update the state and versions
            let path_clone = path.clone();
            let file_infos_clone = Arc::clone(file_infos);
            let hash_info_clone = Arc::clone(hash_info);
            tokio::spawn(async move {
                perform_file_hashing(path_clone.clone(), hash_info_clone).await;
                eprintln!("File closed after write: {}", path_clone);
                if let Some(mut file_info) = file_infos_clone.get_mut(&path_clone) {
                    file_info.version += 1;
                    eprintln!("File {} is ready for hashing.", path_clone);
                    file_info.state = FileState::Closed;
                }
            });
        }
    }
}

fn handle_file_rename(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>, ignore_list: &IgnoreList) {
    if paths.len() != 2 {
        eprintln!("Rename event should have 2 paths: {:?}", paths);
        return;
    }
    let from_path = paths[0].clone();
    let to_path = paths[1].clone();
    if ignore_list.is_ignored(&to_path) {
        // This means the already monitored file is being renamed to something that is ignored
        let hash_info_clone = Arc::clone(hash_info);
        let file_infos_clone = Arc::clone(file_infos);
        tokio::spawn( async move {
            hash_cleanup(&from_path, &file_infos_clone, hash_info_clone).await;
        });
        eprintln!("Ignoring rename event for path: {}", to_path);
        return;
    }

    else if ignore_list.is_ignored(&from_path) {
        // This marks that an ignored file is being renamed to something that should be monitored.
        eprintln!("Handling rename event for paths: {} -> {}", from_path, to_path);
        if !file_infos.contains_key(&to_path) {
            let path_clone = to_path.clone();
            let file_infos_clone = Arc::clone(file_infos);
            let hash_info_clone = Arc::clone(hash_info);
            tokio::spawn(async move {
                perform_file_hashing(path_clone, hash_info_clone).await;
                file_infos_clone.insert(to_path.clone(), FileInfo {
                    file_type: FileType::File,
                    // File can only be renamed when it's closed
                    state: FileState::Closed,
                    version: 1,
                });
            }); 
        }
        return;
    }

    else {
        // This is a normal rename event where the file is being renamed to some other name.
        eprintln!("File renamed from {} to {}", from_path, to_path);
        handle_standard_rename(&from_path, &to_path, file_infos, hash_info);
    }
} 

fn handle_standard_rename(from_path: &String, to_path: &String, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>){
    // Remove the old file from file_infos
    let old_info_opt = file_infos.remove(from_path);
    if let Some((_, old_info)) = old_info_opt{
        file_infos.insert(to_path.clone(), old_info);

        let hash_info_clone = hash_info.clone();
        let to_path_clone = to_path.clone();
        let from_path_clone = from_path.clone();
        tokio::spawn(async move {
            // get old file's hash if available
            if let Ok(mut results) = hash_info_clone.hash_results.try_lock() {
                if let Some(old_hashes) = results.remove(&from_path_clone){
                    // check most recent hashes match
                    match crate::hash::hasher::hash_file(&to_path_clone){
                        Ok(latest_hash_to) => {
                            if latest_hash_to == *old_hashes.last().unwrap() {
                                results.insert(to_path_clone, old_hashes);
                            }
                            else {
                                eprintln!("Warning: Hash mismatch after rename from {} to {}. Removing old history", 
                                from_path_clone, to_path_clone);
                                results.insert(to_path_clone, vec![latest_hash_to]);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error calculating hash for {}: {}", from_path_clone, e);
                        }
                    }

                } else {
                    eprintln!("Error removing old hash for: {}", from_path_clone);

                }
            } else {

            }
        });
    }
    else{
        eprintln!("Received standard rename event but original file absent in file info: {}", from_path);
    }
    

}
fn handle_file_deletion(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>){
    let path = paths[0].clone();
    eprintln!("Handling delete event for path: {}", path);
    let hash_info_clone = Arc::clone(hash_info);
        let file_infos_clone = Arc::clone(file_infos);
        tokio::spawn( async move {
            hash_cleanup(&path, &file_infos_clone, hash_info_clone).await;
        });
        return;
}