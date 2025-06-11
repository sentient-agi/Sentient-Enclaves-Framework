use std::path::Path;
use std::io::Write;
use std::sync::Arc;
use dashmap::DashMap;
use clap::Parser;

mod hash;
mod monitor_module;

use monitor_module::state::FileInfo;
use hash::{storage::HashInfo, retrieve_hash};
use monitor_module::ignore::IgnoreList;
use monitor_module::fs_utils::{ handle_path, set_watch_path };
use monitor_module::debounced_watcher::setup_debounced_watcher;
use async_nats::jetstream;

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

    #[arg(long, default_value = "nats://localhost:4222")] 
    nats_url: String,
    #[arg(long, default_value = "file_hashes")]
    kv_bucket_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let watch_path = Path::new(&args.directory);
    set_watch_path(watch_path.to_path_buf())
        .map_err(|e| format!("Failed to set watch path '{}': {}", watch_path.display(), e))?;

    let ignore_path = Path::new(&args.ignore_file);
    let nats_client = async_nats::connect(&args.nats_url).await
        .map_err(|e| format!("Failed to connect to NATS at {}: {}. Confirm that NATS server is running and JetStream is enabled.", args.nats_url, e))?;
    let js_context = jetstream::new(nats_client);
    let kv_store = js_context.get_key_value(&args.kv_bucket_name).await;

    let kv_store_handle = match kv_store {
        Ok(store) => store,
        Err(_) => {
            let kv_config = jetstream::kv::Config {
                bucket: args.kv_bucket_name.clone(),
                history: 5,
                ..Default::default()
            };
            js_context.create_key_value(kv_config).await?
        }
    };
    println!("Connected to NATS and using KV bucket: {}", args.kv_bucket_name);

    let file_infos: Arc<DashMap<String, FileInfo>> = Arc::new(DashMap::new());
    let hash_infos = Arc::new(HashInfo::new(kv_store_handle));
    
    let mut ignore_list = IgnoreList::new();
    ignore_list.populate_ignore_list(ignore_path);
    
    // Setup file watcher
    let _watcher = setup_debounced_watcher(
        watch_path, 
        Arc::clone(&file_infos), 
        Arc::clone(&hash_infos),
        ignore_list
    ).await?;
    
    // Interactive command loop
    loop {
        println!("Enter path relative to the watched directory to get hash of file");
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

