
use std::path::Path;
use std::io::Write;
use std::sync::Arc;
use dashmap::DashMap;
use clap::Parser;

mod hash;
mod fs_ops;

use fs_ops::state::FileInfo;
use hash::storage::{HashInfo, retrieve_hash};
use fs_ops::ignore::IgnoreList;
use fs_ops::watcher::setup_watcher;
use fs_ops::fs_utils::handle_path;

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
async fn main() -> notify::Result<()> {
    let args = Args::parse();
    let watch_path = Path::new(&args.directory);
    let ignore_path = Path::new(&args.ignore_file);

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo{
        ongoing_tasks: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        hash_results: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    });
    
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(ignore_path);
    
    // Setup file watcher
    let _watcher = setup_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;
    
    // Interactive command loop
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

