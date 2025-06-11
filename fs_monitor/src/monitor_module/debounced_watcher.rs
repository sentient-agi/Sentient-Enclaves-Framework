use notify_debouncer_full::{new_debouncer, notify::*, DebounceEventResult, RecommendedCache};
use std::path::Path;
use tokio::time::Duration;
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::mpsc;
use crate::hash::storage::HashInfo;
use crate::monitor_module::state::FileInfo;
use crate::monitor_module::debounced_events_handler::handle_debounced_event;
use crate::monitor_module::ignore::IgnoreList;
use notify_debouncer_full::Debouncer;

pub async fn setup_debounced_watcher(
    watch_path: &Path, 
    file_infos: Arc<DashMap<String, FileInfo>>,
    hash_infos: Arc<HashInfo>,
    ignore_list: IgnoreList
) -> Result<Debouncer<RecommendedWatcher, RecommendedCache>> {
    let (tx, mut rx) = mpsc::unbounded_channel::<DebounceEventResult>();
    
    // Initialize a debouncer
    let mut debouncer = new_debouncer(Duration::from_secs(1), None, move |res: DebounceEventResult| {
        tx.send(res).expect("Failed to send debounced event");
    })?;

    debouncer.watch(watch_path, RecursiveMode::Recursive)?;
    println!("Started watching {} for changes...", watch_path.display());
    
    // Start a task to handle events
    tokio::spawn(async move {
        while let Some(res) = rx.recv().await {
            match res {
                Ok(debounced_events) => {
                    debounced_events.iter().for_each( |debounced_event| {
                        let _ = handle_debounced_event(debounced_event.clone(), &file_infos, &hash_infos, &ignore_list);
                    }                      
                )},
                Err(errors) => errors.iter().for_each(|error| println!("{error:?}")),
            }
        }
    });

    // Instead of forgetting the watcher, return it
    Ok(debouncer)
} 