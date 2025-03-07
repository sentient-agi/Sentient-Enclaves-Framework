use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sha3::{Digest, Sha3_512};
use std::{
    collections::HashMap,
    io::{self, Read},
    path::Path as StdPath,
    sync::Arc,
};
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<io::Result<Vec<u8>>>>>>,
    results: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    path: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let state = AppState {
        tasks: Arc::new(Mutex::new(HashMap::new())),
        results: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/generate", post(generate_handler))
        .route("/ready/:file_path", get(ready_handler))
        .route("/hash/:file_path", get(hash_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn generate_handler(
    State(state): State<AppState>,
    Json(payload): Json<GenerateRequest>,
) -> impl IntoResponse {
    let path = StdPath::new(&payload.path);

    // Check if the path exists
    let metadata = match tokio::fs::metadata(path).await {
        Ok(meta) => meta,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                format!("Path not found: {}", e),
            )
        }
    };

    let is_dir = metadata.is_dir();
    let state_clone = AppState {
        tasks: Arc::clone(&state.tasks),
        results: Arc::clone(&state.results),
    };

    // Spawn the processing task
    tokio::spawn(async move {
        if let Err(e) = visit_files_recursively(path, state_clone).await {
            eprintln!("Error processing path {}: {}", path.display(), e);
        }
    });

    let message = if is_dir {
        "Started processing directory"
    } else {
        "Started processing file"
    };
    (StatusCode::ACCEPTED, message.to_string())
}

async fn visit_files_recursively(path: &StdPath, state: AppState) -> io::Result<()> {
    if path.is_dir() {
        let mut entries = tokio::fs::read_dir(path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            visit_files_recursively(
                &entry_path,
                AppState {
                    tasks: Arc::clone(&state.tasks),
                    results: Arc::clone(&state.results),
                },
            )
            .await?;
        }
    } else if path.is_file() {
        let file_path = path.to_string_lossy().to_string();
        let handle = tokio::task::spawn_blocking({
            let file_path = file_path.clone();
            move || hash_file(&file_path)
        });

        // Track the task and handle its completion
        state.tasks.lock().await.insert(file_path.clone(), handle);

        let tasks_clone = Arc::clone(&state.tasks);
        let results_clone = Arc::clone(&state.results);
        let file_path_clone = file_path.clone();
        tokio::spawn(async move {
            let result = handle.await;
            let mut tasks = tasks_clone.lock().await;
            tasks.remove(&file_path);

            let mut results = results_clone.lock().await;
            match result {
                Ok(Ok(hash)) => {
                    results.insert(file_path, hash);
                }
                Ok(Err(e)) => {
                    eprintln!("Error hashing file: {}", e);
                }
                Err(e) => {
                    eprintln!("Task panicked: {}", e);
                }
            }
        });
    }
    Ok(())
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

async fn ready_handler(
    AxumPath(file_path): AxumPath<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let results = state.results.lock().await;
    if results.contains_key(&file_path) {
        (StatusCode::OK, "Ready")
    } else {
        let tasks = state.tasks.lock().await;
        if tasks.contains_key(&file_path) {
            (StatusCode::PROCESSING, "Processing")
        } else {
            (StatusCode::NOT_FOUND, "Not found")
        }
    }
}

async fn hash_handler(
    AxumPath(file_path): AxumPath<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let results = state.results.lock().await;
    match results.get(&file_path) {
        Some(hash) => (StatusCode::OK, hex::encode(hash)),
        None => {
            let tasks = state.tasks.lock().await;
            if tasks.contains_key(&file_path) {
                (StatusCode::ACCEPTED, "Processing".to_string())
            } else {
                (StatusCode::NOT_FOUND, "Not found".to_string())
            }
        }
    }
}
