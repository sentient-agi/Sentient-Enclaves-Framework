use notify_debouncer_full::notify::{ Result, EventKind};
use notify_debouncer_full::notify::event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode};
use notify_debouncer_full::DebouncedEvent;
use std::sync::{Arc};
use std::vec;
use dashmap::DashMap;
use crate::hash::storage::{HashInfo, perform_file_hashing};
use crate::fs_ops::state::{FileInfo, FileState, FileType};
use crate::fs_ops::ignore::IgnoreList;
use crate::fs_ops::fs_utils::handle_path;


pub fn handle_debounced_event(debounced_event: DebouncedEvent, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>, ignore_list: &IgnoreList) -> Result<()> {
    let event = debounced_event.event;
    let paths_old: Vec<String> = event.paths.iter()
        .filter_map(|p| p.to_str().map(|s| s.to_string()))
        .collect();

    let mut paths = Vec::new();
    for path in paths_old {
        let path = handle_path(&path);
        paths.push(path);
    }

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
            match rename_mode {
                RenameMode::To => {
                    println!("Rename event for files: {:?} of kind {:?}",paths, rename_mode);
                    handle_file_rename_from_unwatched(paths, file_infos, hash_info);  
                }
                RenameMode::From => {
                    println!("Rename event for files: {:?} of kind {:?}",paths, rename_mode);
                    handle_file_rename_to_unwatched(paths, file_infos, hash_info);

                }
                RenameMode::Both => {
                    println!("Rename event for files: {:?} of kind {:?}",paths, rename_mode);
                    handle_file_rename(paths, file_infos, hash_info, ignore_list);
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
        handle_file_rename_from_unwatched( to_path, file_infos, hash_info)
    }
    // Else this is a standard rename where both old and new paths are tracked
    else{
        eprintln!("File renamed from {} to {}", from_path, to_path);
        handle_file_rename_both_tracked(&from_path, &to_path, file_infos, hash_info)
    }
}

fn handle_file_rename_to_unwatched(from_path: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>){
    // This simply needs to delete the file from being tracked.
    // let frm_path_str = vec![from_path.to_string()];
    handle_file_delete(from_path, file_infos, hash_info);

}

fn handle_file_rename_from_unwatched(to_path: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>){
    // This creates a new entry for the file and performs hashing
    // let to_path_str = vec![to_path.to_string()];    
    handle_file_create(to_path.clone(), file_infos);
    handle_file_save(to_path, file_infos, hash_info);

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
        handle_file_rename_from_unwatched(to_path, file_infos, hash_info);
    }
}

fn handle_file_save(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) {
    if paths.len() != 1 {
        eprintln!("Save on write event has multiple paths: {:?}", paths);
        return;
    }

    let path = paths[0].clone();
    
    if let Some(mut file_info) = file_infos.get_mut(&path) {
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