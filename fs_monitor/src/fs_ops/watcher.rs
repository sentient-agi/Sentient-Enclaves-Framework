use notify::{recommended_watcher, Event, RecursiveMode, Result, Watcher};
use std::path::Path;
use notify::RecommendedWatcher;
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;
use crate::hash::storage::HashInfo;
use crate::fs_ops::state::FileInfo;
use crate::fs_ops::events::handle_event;
use crate::fs_ops::ignore::IgnoreList;

pub async fn setup_watcher(
    watch_path: &Path, 
    file_infos: Arc<DashMap<String, FileInfo>>,
    hash_infos: Arc<HashInfo>,
    ignore_list: IgnoreList
) -> Result<RecommendedWatcher> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Result<Event>>();
    
    // Initialize the watcher
    let mut watcher = recommended_watcher(move |res: Result<Event>| {
        tx.send(res).expect("Failed to send event");
    })?;

    watcher.watch(watch_path, RecursiveMode::Recursive)?;
    println!("Started watching {} for changes...", watch_path.display());
    
    // Start a task to handle events
    tokio::spawn(async move {
        while let Some(res) = rx.recv().await {
            match res {
                Ok(event) => {
                    println!("Event: {:?} for {:?}", event.kind, event.paths);
                    handle_event(event, &file_infos, &hash_infos, &ignore_list).unwrap_or_else(|e| {
                    eprintln!("Error handling event: {}", e);
                });
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
    });

    // Instead of forgetting the watcher, return it
    Ok(watcher)
} 