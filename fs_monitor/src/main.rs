use notify::{recommended_watcher, Event, RecursiveMode, Result, Watcher, EventKind};
use notify::event::{ModifyKind, DataChange, CreateKind, AccessKind, AccessMode, RenameMode};
use tokio::sync::Mutex;
use std::path::Path;
use std::fs;
use sha3::{Digest, Sha3_512};
use std::{
    collections::HashMap,
    io::{self, Read},
    sync::Arc,
};
use dashmap::DashMap;
use clap::Parser;

mod state;
use state::{FileInfo, FileState, FileType};
mod fs_ignore;
use fs_ignore::IgnoreList;
use std::io::Write;


#[derive(Debug, Clone)]
pub struct HashInfo{
    pub ongoing_tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<io::Result<Vec<u8>>>>>>,
    pub hash_results: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
} 

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Filesystem monitoring tool that watches directories and calculates file hashes",
    long_about = None
)]
struct Args {
    /// Directory to watch
    #[arg(short, long, value_name = "DIR", default_value = ".", help = "Specify the directory to watch for file changes.")]
    directory: String,

    /// Path to the ignore file (relative to watched directory or absolute)
    #[arg(short, long, value_name = "FILE", default_value = ".fsignore", help = "Specify the ignore file with patterns to exclude.")]
    ignore_file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let watch_path = Path::new(&args.directory);
    let ignore_path = Path::new(&args.ignore_file);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Result<Event>>();

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());

    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(Mutex::new(HashMap::new())),
        hash_results: Arc::new(Mutex::new(HashMap::new())),
    });
    
    // Clones for the closure
    let file_infos_clone = Arc::clone(&file_infos);
    let hash_infos_clone = Arc::clone(&hash_infos);
    
    // Initialize the watcher
    let mut watcher = recommended_watcher(move |res: Result<Event>| {
        tx.send(res).expect("Failed to send event");
    })?;

    watcher.watch(watch_path, RecursiveMode::Recursive)?;
    println!("Started watching {} for changes...", args.directory);
    
    let mut ignore_list = IgnoreList::new();

    ignore_list.populate_ignore_list(ignore_path);
    
    // Start a task to handle events
    tokio::spawn(async move {
        while let Some(res) = rx.recv().await {
            match res {
                Ok(event) => {
                handle_event(event, &file_infos_clone,&hash_infos_clone, &ignore_list).unwrap_or_else(|e| {
                    eprintln!("Error handling event: {}", e);
                });
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
});

loop {
    println!("Enter path relative to current working directory to get hash of file");
    print!(">>> ");
    std::io::stdout().flush().unwrap();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let path = input.trim();

    let path = handle_path(path);
    println!("path: {}", path);
    match retrieve_hash(&path, &file_infos, &hash_infos).await {
         Ok(hash_string) => println!("Hash for {}: {}", path, hash_string),
         Err(e) => eprintln!("Error retrieving hash for {}: {}", path, e),
    }
    println!("================================================");
}
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

async fn retrieve_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) -> io::Result<String> {
    let metadata = fs::metadata(path).map_err(|e| {
        io::Error::new(e.kind(), format!("Failed to get metadata for '{}': {}", path, e))
    })?;

    if metadata.is_dir() {
        let mut files = Vec::new();
        for entry in file_infos.iter() {
            let file_path = entry.key();
            if file_path.starts_with(path) && !file_path.ends_with('/') {
                files.push(file_path.clone());
            }
        }
        
        // Sort files for consistent hash results
        files.sort();

        let mut dir_hasher = Sha3_512::new();
        for file_path in files {
            let file_hash = retrieve_file_hash(&file_path, file_infos, hash_info).await?;
            dir_hasher.update(file_hash);
        }

        Ok(bytes_to_hex(&dir_hasher.finalize()))
    } else {
        let hash_bytes = retrieve_file_hash(path, file_infos, hash_info).await?;
        Ok(bytes_to_hex(&hash_bytes))
    }
}

async fn retrieve_file_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>) -> io::Result<Vec<u8>> {
    let file_info = file_infos.get(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "File not tracked"))?;

    if file_info.state != FileState::Closed {
        return Err(io::Error::new(io::ErrorKind::Other, "File is yet to be closed"));
    }

    let results_map = hash_info.hash_results.lock().await;
    let hash_vector = results_map.get(path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No hashes recorded"))?;

    let version = file_info.version as usize;
    if hash_vector.len() != version {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Latest hash is not available"));
    }

    hash_vector.last()
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No hash available"))
}

fn handle_event(event: Event, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>, ignore_list: &IgnoreList) -> Result<()> {
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
            eprintln!("File closed after write: {}", path);
            file_info.version += 1;
            eprintln!("File {} is ready for hashing.", path);
            file_info.state = FileState::Closed;
            tokio::spawn(perform_file_hashing(path.clone(), Arc::clone(hash_info)));
        }
    }
}


// TODO: Handle rename events
fn handle_file_rename(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfo>, ignore_list: &IgnoreList) {
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
        // We need to update the entry in file_infos for the new path. The hash of the file should be recalculated?
        // Currently this event is also ignored.
        // Confirm that the file hash is not changed. Confirm the new hash is equal to the latest hash already stored
        eprintln!("File renamed from {} to {}", from_path, to_path);
        // Copy the file_info state for new file
        if let Some(file_info) = file_infos.get(&from_path).map(|info| info.clone()) {
            let from_path_clone = from_path.clone();
            let to_path_clone = to_path.clone();
            let file_infos_clone = Arc::clone(file_infos);
            let hash_info_clone = Arc::clone(hash_info);
            
            tokio::spawn(async move {
                // Calculate hash for the new file
                match hash_file(&to_path_clone) {
                    Ok(latest_hash) => {
                        // Try to get the old hash for comparison
                        match retrieve_file_hash(&from_path_clone, &file_infos_clone, &hash_info_clone).await {
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

// This has become very complicated cause of MT-runtime incorporation.
// Can we just await the task instead?
async fn perform_file_hashing(path: String, hash_info: Arc<HashInfo>){
    let file_path = path;
    let handle = tokio::task::spawn_blocking({
        let file_path = file_path.clone();
        move || hash_file(&file_path)
    });

    hash_info.ongoing_tasks.lock().await.insert(file_path.clone(), handle);

    let ongoing_tasks = Arc::clone(&hash_info.ongoing_tasks);
    let hash_results = Arc::clone(&hash_info.hash_results);
    let file_path_clone = file_path.clone();
    
    tokio::spawn(async move {
        let task_result = {
            let mut tasks = ongoing_tasks.lock().await;
            if let Some(handle) = tasks.get_mut(&file_path_clone) {
                Some(async { handle.await }.await)
            } else {
                None
            }
        };

        if let Some(result) = task_result {
            match result {
                Ok(Ok(hash)) => {
                    let mut results = hash_results.lock().await;
                    let mut hashes = results.get(&file_path_clone).unwrap_or(&Vec::new()).clone();
                    hashes.push(hash);
                    results.insert(file_path_clone.clone(), hashes);
                }
                Ok(Err(e)) => {
                    eprintln!("Error hashing file: {}", e);
                }
                Err(e) => {
                    eprintln!("Task panicked: {}", e);
                }
            }
            // Remove the task from HashMap after awaiting it (after it completes)
            let mut tasks = ongoing_tasks.lock().await;
            tasks.remove(&file_path_clone);
        }
    });


}

fn hash_file(file_path: &str) -> io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(file_path)?;
    let mut hasher = Sha3_512::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_vec())
}


fn handle_path(path: &str) -> String {
    // check if path is absolute
    // if it is then make it relative
    // with respect to the current working directory
    if path.starts_with("/") {
        let current_dir = std::env::current_dir().unwrap();
        let relative_path = Path::new(&path).strip_prefix(current_dir).unwrap();
        // if there is ./ in the path then remove it
        let relative_path = if relative_path.starts_with("./") {
            relative_path.strip_prefix("./").unwrap()
        } else {
            relative_path
        };
        // update the path with the relative path
        let path = relative_path.to_str().unwrap();
        // eprintln!("Relative path: {}", path);
        path.to_string()
    }
    else {
        path.to_string()
    }    
}