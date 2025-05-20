use notify_debouncer_full::notify::{ Result, EventKind};
use notify_debouncer_full::notify::event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode};
use notify_debouncer_full::DebouncedEvent;
use std::sync::{Arc};
use std::vec;
use dashmap::DashMap;
use crate::hash::storage::{HashInfo, perform_file_hashing};
use crate::monitor_module::state::{FileInfo, FileState, FileType};
use crate::monitor_module::ignore::IgnoreList;
use crate::monitor_module::fs_utils::{handle_path, is_directory, walk_directory};
use std::io;


pub fn handle_debounced_event(debounced_event: DebouncedEvent, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>, ignore_list: &IgnoreList) -> Result<()> {
    let event = debounced_event.event;
    let paths: Vec<String> = event.paths.iter()
        .filter_map(|p| p.to_str().map(|s| handle_path(s)))
        .collect();

    // Return early if there are no paths or if all paths should be ignored
    if paths.is_empty() || paths.iter().all(|path| ignore_list.is_ignored(path)) {
        return Ok(());
    }

    match event.kind {
        EventKind::Create(kind) => {
            match kind {
                CreateKind::File => {
                    println!("Create event for file: {:?}", paths);
                    handle_file_create(paths, file_infos);
                },
                CreateKind::Folder => {
                    // With a flat structure we need not worry about folders.
                    // This event can simply be ignored.
                    println!("Create event for Folder: {:?}", paths);
                },
                _ => {}
            }
        }
        EventKind::Remove(kind) => {
            match kind {
                RemoveKind::File => {
                    println!("Remove event for file: {:?}", paths);
                    handle_file_delete(paths.clone(), &file_infos, &hash_info);
                },
                RemoveKind::Folder => {
                    handle_directory_delete(paths.clone(), &file_infos, &hash_info);
                    println!("Remove event for Folder: {:?}", paths);
                },
                _ => {}
            }
        }
        
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
            println!("File save event for file: {:?}",paths);
            handle_file_save(paths, file_infos, hash_info);
        }
        
        EventKind::Modify(ModifyKind::Data(DataChange::Any) ) => {
            println!("Modify event for file: {:?}",paths);
           handle_file_modify(paths, file_infos); 
        }
        
        EventKind::Modify(ModifyKind::Name(rename_mode)) => {
            let path = paths[0].clone();
            match rename_mode {
                RenameMode::To => {
                    println!("Rename event for: {:?} of kind {:?}",paths, rename_mode);
                    handle_rename_to_watched(paths, file_infos, hash_info);


                }
                RenameMode::From => {
                    println!("Rename event for: {:?} of kind {:?}",paths, rename_mode);
                    handle_directory_delete(vec![path], file_infos, hash_info);
                    // if is_directory(&path){
                    //     eprintln!("Triggered Directory Rename");
                    //     handle_directory_delete(vec![path], file_infos, hash_info);

                    // } else {
                    //     eprintln!("Triggered file Rename");
                    //     handle_file_rename_to_unwatched(paths, file_infos, hash_info);
                    // }

                }
                RenameMode::Both => {
                    println!("Rename event for: {:?} of kind {:?}",paths, rename_mode);
                    // Here, from_path wouldn't exist. Instead use to_path for verifying
                    let to_path = paths[1].clone();
                    if is_directory(&to_path){
                        handle_directory_rename(paths, file_infos, hash_info, ignore_list)

                    } else {
                        handle_file_rename(paths, file_infos, hash_info, ignore_list);
                    }
                    
                }
                _ => {

                }
                
            }
        }
        
        _ => {
            // eprintln!("Unhandled event kind: {:?} for paths: {:?} ",event.kind, paths);
        }
    }

    Ok(())
}

// File specific functions
fn handle_file_create(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    
    if paths.len() != 1 {
        eprintln!("Create event has multiple paths: {:?}", paths);
        return;
    }

    let path = paths[0].clone();
    file_infos.insert(path.clone(), FileInfo {
            file_type: FileType::File,
            state: FileState::Created,
            version: 0,
    });
}

fn handle_file_modify(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    if paths.len() != 1 {
        eprintln!("Modify event has multiple paths: {:?}", paths);
        return;
    }

    let path = paths[0].clone();
    
    if let Some(mut file_info) = file_infos.get_mut(&path) {
        file_info.state = FileState::Modified;
    }
}


fn handle_file_rename(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>, ignore_list: &IgnoreList){
    if paths.len() != 2 {
        eprintln!("Rename event should have 2 paths: {:?}", paths);
        return;
    }

    let from_path = paths[0].clone();
    let to_path = paths[1].clone();

    // Already  handled case of when both paths are ignored.

    // Check if renamed into an unwatched file
    if ignore_list.is_ignored(&to_path) {
        eprintln!("File renamed from {} to ignored file: {}", from_path, to_path);
        let from_path = vec![from_path];
        handle_file_rename_to_unwatched(from_path, file_infos, hash_info)
    }
    // Check if renamed from an unwatched directory
    else if ignore_list.is_ignored(&from_path) {
        eprintln!("File renamed from ignored file: {} to  {}", from_path, to_path);
        let to_path = vec![to_path];
        handle_rename_to_watched( to_path, file_infos, hash_info)
    }
    // Else this is a standard rename where both old and new paths are tracked
    else{
        eprintln!("File renamed from {} to {}", from_path, to_path);
        handle_file_rename_both_tracked(&from_path, &to_path, file_infos, hash_info)
    }
}

fn handle_directory_rename(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>, ignore_list: &IgnoreList){
    if paths.len() != 2 {
        eprintln!("Rename event should have 2 paths: {:?}", paths);
        return;
    }

    let from_path = paths[0].clone();
    let to_path = paths[1].clone();

    // Check if renamed into an unwatched file
    if ignore_list.is_ignored(&to_path) {
        eprintln!("Directory renamed from {} to ignored Directory: {}", from_path, to_path);
        let from_path = vec![from_path];
        handle_directory_delete(from_path, file_infos, hash_info);
    }
    // Check if renamed from an unwatched directory
    else if ignore_list.is_ignored(&from_path) {
        eprintln!("Directory renamed from ignored Directory: {} to  {}", from_path, to_path);
        let to_path = vec![to_path];
        handle_rename_to_watched( to_path, file_infos, hash_info)
    }
    // Else this is a standard rename where both old and new paths are tracked
    else{
        eprintln!("Directory renamed from {} to {}", from_path, to_path);
        handle_directory_rename_both_tracked(&from_path, &to_path, file_infos, hash_info)
    }
}



fn handle_file_rename_to_unwatched(from_path: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>){
    // This simply needs to delete the file from being tracked.
    // let frm_path_str = vec![from_path.to_string()];
    handle_file_delete(from_path, file_infos, hash_info);

}


fn handle_file_rename_both_tracked(from_path: &String, to_path: &String, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>){
    // Transfer Old file's history
    let file_info_opt = file_infos.remove(from_path);
    if let Some((_, old_info)) = file_info_opt {
        // Insert the file info with the new path
        file_infos.insert(to_path.clone(), old_info);
        
        let hash_info = Arc::clone(hash_info);
        // let file_infos = Arc::clone(file_infos);
        let to_path = to_path.clone();
        let from_path = from_path.clone();
        
        tokio::spawn(async move {
            // Transfer hash history
            {
                let mut hash_results = hash_info.hash_results.lock().await;
                if let Some(hash_history) = hash_results.remove(&from_path) {
                    hash_results.insert(to_path, hash_history);
                }
            }
            // Perform hashing and update file info
            // This completely depends on policy.
            // For now this should be removed.
            // perform_file_hashing(to_path.clone(), hash_info).await;
            // if let Some(mut file_info) = file_infos.get_mut(&to_path) {
            //     file_info.version += 1;
            //     file_info.state = FileState::Closed;
            // }
        });
    } else {
        // New entry received before old entry is created. 
        // Can this even happen with debouncer?
        // For now handle it similar to when a new file is created
        eprint!("Old file: {} renamed to: {}. But no entry found for :{}", from_path, to_path, from_path);
        let to_path = vec![to_path.clone()];
        handle_rename_to_watched( to_path, file_infos, hash_info)
    }
}

fn handle_directory_rename_both_tracked(from_path: &String, to_path: &String, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>){
    let collected_files = collect_files_in_directory(from_path.to_string(), file_infos);
    let path_pairs: Vec<(String, String)> = collected_files.iter().map(|old_path| {
        let new_path = old_path.replacen(from_path, &to_path, 1);
        eprintln!("File renamed from: {:?} to {:?}", old_path, new_path);
        (old_path.clone(), new_path)
    }).collect();
    
    for (old_path, new_path) in &path_pairs {
        if let Some((_, file_info)) = file_infos.remove(old_path) {
            file_infos.insert(new_path.clone(), file_info);
        }
    }

    let hash_info_clone = Arc::clone(hash_info);
    let path_pairs_clone = path_pairs.clone();

    // Transfer hash history
    tokio::spawn( async move {
        let mut hash_results = hash_info_clone.hash_results.lock().await;
        for (old_path, new_path) in path_pairs_clone {
            if let Some(hash_history) = hash_results.remove(&old_path) {
                hash_results.insert(new_path, hash_history);
            }
        }
    });
}


fn handle_file_save(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) {
    if paths.len() != 1 {
        eprintln!("Save on write event has multiple paths: {:?}", paths);
        return;
    }

    let path = paths[0].clone();
    
    if let Some(_file_info) = file_infos.get_mut(&path) {
        // Calculate hash first and then update the state and versions
        let path_cloned = path.clone();
        let file_infos_cloned: Arc<DashMap<String, FileInfo>> = Arc::clone(file_infos);
        let hash_info_cloned = Arc::clone(hash_info);
        eprintln!("File : {} closed after write and is ready for hashing", path_cloned);

        // Perform the update in the background thread to return immediately
        tokio::spawn(async move {
            perform_file_hashing(path_cloned.clone(), hash_info_cloned).await;
            if let Some(mut file_info) = file_infos_cloned.get_mut(&path_cloned) {
                file_info.version += 1;
                file_info.state = FileState::Closed;
            }
        });
    }
}

fn handle_file_delete(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>){
    let path = paths[0].clone();
    eprintln!("Handling delete event for path: {}", path);
    let hash_info_clone = Arc::clone(hash_info);
    
    // clean-up the file metadata
    file_infos.remove(&path);
    
    // In the background thread, clean-up the hash
    tokio::spawn( async move {
        // Clean-up the hash
        let mut hash_results = hash_info_clone.hash_results.lock().await;
        hash_results.remove(&path);
    });
}

fn collect_files_in_directory(dir_path: String, file_infos: &Arc<DashMap<String, FileInfo>>) -> Vec<String> {
    file_infos.iter()
        .filter_map(|ref_multi| {
            let path = ref_multi.key().to_string();
            if path.starts_with(&dir_path) {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

fn handle_directory_delete(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>){
    let directory_path = paths[0].clone();

    let collected_files = collect_files_in_directory(directory_path, file_infos);

    eprint!("Collected Files: {:?}", collected_files);
    
    // First remove the maintained state synchronously
    for file_path in collected_files.iter(){
       file_infos.remove(file_path);
    }

    let hash_info_clone = Arc::clone(hash_info);
    // In async way remove the hash results
    tokio::spawn( async move {
        let mut hash_results = hash_info_clone.hash_results.lock().await;
        for file_path in collected_files.iter() {
            // Clean-up the hash
            hash_results.remove(file_path);
        }
    });

}

fn handle_rename_to_watched(to_path: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) {
    if to_path.is_empty() { return; }
    
    let path = &to_path[0];
    
    if is_directory(path) {
        let dir_path = path.clone();
        let file_infos_clone = Arc::clone(file_infos);
        let hash_info_clone = Arc::clone(hash_info);
        
        tokio::spawn(async move {
            match walk_directory(&dir_path) {
                Ok(files) => {
                    for file_path in &files {
                        file_infos_clone.insert(file_path.clone(), FileInfo {
                            file_type: FileType::File,
                            state: FileState::Closed,
                            version: 0,
                        });
                        
                        perform_file_hashing(file_path.to_string(), Arc::clone(&hash_info_clone)).await;
                    }
                    eprintln!("Processed directory {}: added {} files to tracking", dir_path, files.len());
                },
                Err(e) => {
                    eprintln!("Error collecting files from {}: {}", dir_path, e);
                }
            }
        });
    } else {
        // Create and hash the file
        file_infos.insert(path.clone(), FileInfo {
            file_type: FileType::File,
            state: FileState::Created,
            version: 0,
        });
        
        let path_clone = path.clone();
        let hash_info_clone = Arc::clone(hash_info);
        let file_infos_clone = Arc::clone(file_infos);
        
        tokio::spawn(async move {
            perform_file_hashing(path_clone.clone(), hash_info_clone).await;
            if let Some(mut file_info) = file_infos_clone.get_mut(&path_clone) {
                file_info.version += 1;
                file_info.state = FileState::Closed;
            }
        });
    }
}