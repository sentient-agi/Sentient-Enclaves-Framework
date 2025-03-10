use axum::{
    extract::{Path as AxumPath, Query, State},
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
use std::future::Future;
use std::pin::Pin;
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
    let state = Arc::new(AppState {
        tasks: Arc::new(Mutex::new(HashMap::new())),
        results: Arc::new(Mutex::new(HashMap::new())),
    });

    let app = Router::new()
        .route("/generate", post(generate_handler))
        .route("/ready/", get(ready_handler))
        .route("/hash/", get(hash_handler))
        .route("/echo/", get(echo))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8888").await?;
    axum_server::Server::from_tcp(listener.into_std()?).serve(app.into_make_service()).await?;

    Ok(())
}

async fn generate_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<GenerateRequest>,
) -> impl IntoResponse {
    let path_str = payload.path.clone();
    let path = StdPath::new(&path_str);

    // Check if the path exists
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                format!("Path not found: {}", e),
            );
        }
    };

    let is_dir = metadata.is_dir();

    let state_clone = state.clone();

    // Spawn the processing task
    tokio::spawn(async move {
        let path_buf = StdPath::new(&path_str).to_path_buf();
        if let Err(e) = visit_files_recursively(&path_buf, state_clone).await {
            eprintln!("Error processing path {}: {}", path_buf.display(), e);
        }
    });

    let message = if is_dir {
        "Started processing directory"
    } else {
        "Started processing file"
    };
    (StatusCode::ACCEPTED, message.to_string())
}

fn visit_files_recursively<'a>(
    path: &'a StdPath,
    state: Arc<AppState>
) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + Sync + 'a>> {
    Box::pin(async move {
        if path.is_dir() {
            let mut entries = tokio::fs::read_dir(path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let entry_path = entry.path();
                visit_files_recursively(&entry_path, Arc::clone(&state)).await?;
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
                let task_result = {
                    let mut tasks = tasks_clone.lock().await;
                    if let Some(handle) = tasks.get_mut(&file_path_clone) {
                        Some(async { handle.await }.await)
                    } else {
                        None
                    }
                };

                if let Some(result) = task_result {
                    match result {
                        Ok(Ok(hash)) => {
                            let mut results = results_clone.lock().await;
                            results.insert(file_path_clone.clone(), hash);
                        }
                        Ok(Err(e)) => {
                            eprintln!("Error hashing file: {}", e);
                        }
                        Err(e) => {
                            eprintln!("Task panicked: {}", e);
                        }
                    }

                    // Remove the task from HashMap after awaiting it (after it completes)
                    let mut tasks = tasks_clone.lock().await;
                    tasks.remove(&file_path_clone);
                }
            });

            // Yield to allow other async tasks to make progress
            tokio::task::yield_now().await;
        }
        Ok(())
    })
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
    // AxumPath(file_path): AxumPath<String>,
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let file_path = query_params.get("path").unwrap().to_owned();
    let results = state.results.lock().await;
    if results.contains_key(&file_path) {
        (StatusCode::OK, "Ready".to_string())
    } else {
        let tasks = state.tasks.lock().await;
        if tasks.contains_key(&file_path) {
            (StatusCode::PROCESSING, "Processing".to_string())
        } else {
            (StatusCode::NOT_FOUND, "Not found".to_string())
        }
    }
}

async fn hash_handler(
    // AxumPath(file_path): AxumPath<String>,
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let file_path = query_params.get("path").unwrap().to_owned();
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

/// Testing endpoint handler for various purposes
async fn echo(
    Query(query_params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse {
    let file_path = query_params.get("path").unwrap().to_owned();
    println!("File path: {:?}", file_path);
    (StatusCode::OK, file_path)
}
