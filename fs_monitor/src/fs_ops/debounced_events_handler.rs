use notify_debouncer_full::notify::{Event, Result, EventKind};
use notify_debouncer_full::notify::event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode};
use notify_debouncer_full::DebouncedEvent;
use std::sync::{Arc, RwLock};
use dashmap::DashMap;
use crate::hash::storage::{HashInfo, perform_file_hashing, hash_cleanup};
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
                },
                CreateKind::Folder => {
                    println!("Create event for Folder: {:?}", paths);
                },
                _ => {}
            }
            // handle_file_creation(paths.clone(), &file_infos);
        }
        EventKind::Remove(kind) => {
            match kind {
                RemoveKind::File => {
                    println!("Remove event for file: {:?}", paths);
                },
                RemoveKind::Folder => {
                    println!("Remove event for Folder: {:?}", paths);
                },
                _ => {}
            }
            // handle_file_deletion(paths.clone(), &file_infos, &hash_info);
        }
        
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
            println!("File save event for file: {:?}",paths);
            // handle_file_save_on_write(paths.clone(), &file_infos, &hash_info);
        }
        
        EventKind::Modify(ModifyKind::Data(DataChange::Any) ) => {
            println!("Modify event for file: {:?}",paths);
        //    handle_file_data_modification(paths.clone(), &file_infos); 
        }
        
        EventKind::Modify(ModifyKind::Name(rename_mode)) => {
            println!("Rename event for files: {:?}",paths);
        }
        
        _ => {
            // eprintln!("Unhandled event kind: {:?} for paths: {:?} ",event.kind, paths);
        }
    }

    Ok(())
}

// File specific functions
fn handle_file_creation(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    
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

fn handle_file_modification(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>) {
    if paths.len() != 1 {
        eprintln!("Modify event has multiple paths: {:?}", paths);
        return;
    }

    let path = paths[0].clone();
    
    if let Some(mut file_info) = file_infos.get_mut(&path) {
        file_info.state = FileState::Modified;
    }
}

fn handle_file_renames(){

}

fn handle_file_save(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) {
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