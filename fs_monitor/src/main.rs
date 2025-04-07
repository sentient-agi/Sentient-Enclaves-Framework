use notify::{recommended_watcher, Event, RecursiveMode, Result, Watcher, EventKind};
use notify::event::{ModifyKind, DataChange, CreateKind, AccessKind, AccessMode, RenameMode};
use tokio::sync::Mutex;
use std::sync::mpsc;
use std::path::Path;
use std::fs;
use sha3::{Digest, Sha3_512};
use std::{
    collections::HashMap,
    io::{self, Read},
    sync::Arc,
};
use std::thread;
use dashmap::DashMap;

mod state;
use state::{FileInfo, FileState, FileType};
mod fs_ignore;
use fs_ignore::IgnoreList;
use std::io::Write;


#[derive(Debug, Clone)]
pub struct HashInfoNew{
    pub ongoing_tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<io::Result<Vec<u8>>>>>>,
    pub hash_results: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
} 

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());

    let hash_info = Arc::new(HashInfoNew{
        ongoing_tasks: Arc::new(Mutex::new(HashMap::new())),
        hash_results: Arc::new(Mutex::new(HashMap::new())),
    });
    
    // Clone for the closure
    let file_infos_clone = Arc::clone(&file_infos);
    
    // Initialize the watcher
    let mut watcher = recommended_watcher(move |res: Result<Event>| {
        tx.send(res).expect("Failed to send event");
    })?;

    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;
    println!("Started watching current directory for changes...");
    
    let mut ignore_list = IgnoreList::new();

    // TODO: Remove hardcoding of path. Make path relative to the src directory?
    ignore_list.populate_ignore_list("/home/ec2-user/pipeline-tee.rs/fs_monitor/fs_ignore");
    
    // Start a thread to handle events
    thread::spawn(move || {
        for res in rx {
            match res {
                Ok(event) => {
                handle_event(event, &file_infos_clone,&hash_info, &ignore_list).unwrap_or_else(|e| {
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
    // println!("path: {}", path);
    // let hash = retrieve_hash(&path, &file_infos)?;
    // println!("{}", hash);
    println!("================================================");
}
}


async fn retrieve_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfoNew>) -> Result<String> {
    // Check if the requested file is a directory
    if fs::metadata(path)?.is_dir(){
        let mut hash_final = String::new();
        for file in file_infos.iter(){
            let file_name = file.key();
            if file_name.starts_with(path){
                hash_final.push_str(&retrieve_file_hash(&file_name, file_infos, hash_info).await.unwrap());
            }
        }
        Ok(hash_final)
    }
    else{
        retrieve_file_hash(path, file_infos, hash_info).await
    }
}

async fn retrieve_file_hash(path: &str, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfoNew>) -> Result<String> {
    
    // Check that file modification is done
    if !file_infos.contains_key(path) || 
    (file_infos.contains_key(path) && file_infos.get(path).unwrap().state != FileState::Closed){
        Ok("Hash Unavailable".to_string())
    }
    else{
        // Get latest version and then check if that hash is available
        let latest_version = file_infos.get(path).unwrap().version;
        let hashes = hash_info.hash_results.lock().await;
        let hash_vector = hashes.get(path).unwrap();
        if hash_vector.len() != latest_version.try_into().unwrap(){
            return Ok("Hash Unavailable".to_string());
        }
        else{
            // Fetch the latest version's hash
            let latest_hash = hash_vector.get(latest_version).unwrap();
            let hash_string = match String::from_utf8(latest_hash.to_vec()){
                Ok(v) => v,
                Err(e) => panic!("Invalid UTF-8 Sequence: {}", e),
            };
            Ok(hash_string)
        }
        
    }
}
fn handle_event(event: Event, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfoNew>, ignore_list: &IgnoreList) -> Result<()> {
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
            // eprintln!("#Unhandled event {:?} for: {}", event.kind, path);
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
    // eprintln!("File modified: {}", path);
    if let Some(mut file_info) = file_infos.get_mut(&path) {
        if file_info.file_type == FileType::File {
            file_info.state = FileState::Modified;
            // TODO: Also reset hash here
        }
    }
}

fn handle_file_save_on_write(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfoNew>) {
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
fn handle_file_rename(paths: Vec<String>, file_infos: &Arc<DashMap<String, FileInfo>>, hash_info: &Arc<HashInfoNew>, ignore_list: &IgnoreList) {
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
                state: FileState::Renamed,
                version: 0,
            });
            tokio::spawn(perform_file_hashing(to_path.clone(), Arc::clone(hash_info)));
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

async fn perform_file_hashing(path: String, hash_info: Arc<HashInfoNew>){
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