use notify_debouncer_full::notify::{Event, Result, EventKind};
use notify_debouncer_full::notify::event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode};
use notify_debouncer_full::DebouncedEvent;
use std::sync::Arc;
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
        EventKind::Create(CreateKind::File) => {
            println!("Create event for file: {:?}",paths);
            // handle_file_creation(paths.clone(), &file_infos);
        }
        // EventKind::Create(CreateKind::Folder) => {
        //     handle_directory_creation(paths.clone(), &file_infos);
        // }
        EventKind::Modify(ModifyKind::Data(DataChange::Any) ) => {
            println!("Modify event for file: {:?}",paths);
        //    handle_file_data_modification(paths.clone(), &file_infos); 
        }
        EventKind::Remove(RemoveKind::File) => {
            println!("Remove event for file: {:?}",paths);
            // handle_file_deletion(paths.clone(), &file_infos, &hash_info);
        }
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
            println!("File save event for file: {:?}",paths);
            // handle_file_save_on_write(paths.clone(), &file_infos, &hash_info);
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