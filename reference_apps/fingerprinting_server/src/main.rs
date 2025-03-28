use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{post, get},
    Router,
};
use std::net::SocketAddr;
use fingerprinting_server::{FingerprintRequest, GenerateFingerprintRequest, ServerConfig};
use std::sync::{Arc, Mutex};
use serde_json::json;
use axum::debug_handler;
use std::hash::{Hash, Hasher};
use lazy_static::lazy_static;

lazy_static! {
    static ref SERVER_CONFIG: ServerConfig = ServerConfig::new().unwrap_or_else(|e| {
        eprintln!("Failed to load server config: {}", e);
        std::process::exit(1);
    });
}

#[derive(Debug)]
struct AppState {
    fingerprinting: bool,
    generating_fingerprints: bool,
    config_hash: Option<String>,
}

#[tokio::main]
async fn main() {
    // Initialize shared state
    let state = Arc::new(Mutex::new(AppState {
        fingerprinting: false,
        generating_fingerprints: false,
        config_hash: None,
    }));

    // Clone state for use in handlers
    let state_clone = state.clone();

    // Build the application with routes
    let app = Router::new()
        .route("/fingerprint", post(fingerprint_handler))
        .route("/generate_fingerprints", post(generate_fingerprints_handler))
        .route("/status", get(status_handler))
        .with_state(state_clone);

    // Specify the address to run the server on
    let port:u16 = SERVER_CONFIG.port;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Server running at http://{}", addr);

    // Run the server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn generate_config_hash(payload: &FingerprintRequest) -> String {
    // For all the fields in the payload, generate a hash
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for field in payload.to_args() {
        field.hash(&mut hasher);
    }
    format!("{:?}", hasher.finish())
}

async fn status_handler(State(state): State<Arc<Mutex<AppState>>>) -> impl IntoResponse {
    let app_state = state.lock().unwrap();
    println!("App state: {:?}", app_state);

    if app_state.fingerprinting {
        let response = json!({
            "status": "In progress",
            "operation": "fingerprint",
            "config_hash": app_state.config_hash.clone().unwrap_or_default(),
        });
        serde_json::to_string(&response).unwrap().into_response()
    } else if app_state.generating_fingerprints {
        let response = json!({
            "status": "In progress",
            "operation": "generate_fingerprints",
            "config_hash": app_state.config_hash.clone().unwrap_or_default(),
        });
        serde_json::to_string(&response).unwrap().into_response()
    } else {
        let response = json!({
            "status": "Available",
        });
        serde_json::to_string(&response).unwrap().into_response()
    }
}

#[debug_handler]
async fn fingerprint_handler(
    State(state): State<Arc<Mutex<AppState>>>,
    Json(payload): Json<FingerprintRequest>,
) -> impl IntoResponse {

    let mut app_state = state.lock().unwrap();
    if app_state.fingerprinting {
        let response = json!({
            "status": "In progress",
            "operation": "fingerprint",
            "config_hash": app_state.config_hash.clone().unwrap_or_default(),
        });
        return  Json(response);
    } else if app_state.generating_fingerprints {
        let response = json!({
            "status": "In progress",
            "operation": "generate_fingerprints",
            "config_hash": app_state.config_hash.clone().unwrap_or_default(),
        });
        return Json(response);
    }
    app_state.fingerprinting = true;
    app_state.config_hash = Some(generate_config_hash(&payload));

    // Clone state for the spawned task
    let state_for_thread = state.clone();
    let hash = app_state.config_hash.clone().unwrap_or_default();
    // Spawn the fingerprinting task
    let _thread = tokio::spawn(async move {
        let deepspeed_command = format!("{}/deepspeed", &SERVER_CONFIG.deepspeed_dir);
        println!("Deepspeed command: {}", deepspeed_command);
        let mut command = tokio::process::Command::new(deepspeed_command);
        command
            .current_dir(&SERVER_CONFIG.fingerprinting_source_dir)
            .arg("--no_local_rank")
            .arg("finetune_multigpu.py");

        for arg in payload.to_args() {
            command.arg(arg);
        }

        match command.spawn() {
            Ok(mut child) => {
                match child.wait().await {
                    Ok(exit_status) => {
                        println!("Fingerprinting process exited with: {}", exit_status);
                        let mut app_state = state_for_thread.lock().unwrap();
                        app_state.fingerprinting = false;
                        app_state.config_hash = None;
                    }
                    Err(e) => {
                        eprintln!("Failed to wait on fingerprinting process: {}", e);
                        let mut app_state = state_for_thread.lock().unwrap();
                        app_state.fingerprinting = false;
                        app_state.config_hash = None;
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to spawn fingerprinting process: {}", e);
                let mut app_state = state_for_thread.lock().unwrap();
                app_state.fingerprinting = false;
                app_state.config_hash = None;
            }
        }
    });

    let response = json!({
        "status": "Started",
        "operation": "fingerprint",
        "config_hash": hash,
    });
    Json(response)

}

#[debug_handler]
   async fn generate_fingerprints_handler(
       State(state): State<Arc<Mutex<AppState>>>,
       Json(payload): Json<GenerateFingerprintRequest>
   ) -> impl IntoResponse {
    let mut app_state = state.lock().unwrap();
    if app_state.fingerprinting {
        let response = json!({
            "operation": "fingerprint",
            "status": "In progress",
            "config_hash": app_state.config_hash.clone().unwrap_or_default(),
        });
        return Json(response);
    } else if app_state.generating_fingerprints {
        let response = json!({
            "operation": "generate_fingerprints",
            "status": "In progress",
            "config_hash": app_state.config_hash.clone().unwrap_or_default(),
        });
        return Json(response);
    }
    app_state.generating_fingerprints = true;
    app_state.config_hash = Some("SOME_HASH for generate_fingerprint".to_string());

    // Clone state for the spawned task
    let state_for_thread = state.clone();
    let hash = app_state.config_hash.clone().unwrap_or_default();
    // Spawn the fingerprinting task
    let _thread = tokio::spawn(async move {
        // If fingerprint file path already exists, delete it
        if std::path::Path::new(&payload.output_file).exists() {
            println!("Fingerprint file already exists, deleting it");
            std::fs::remove_file(&payload.output_file).unwrap();
        }
        let deepspeed_command = format!("{}/deepspeed", &SERVER_CONFIG.deepspeed_dir);
        println!("Deepspeed command: {}", deepspeed_command);
        let mut command = tokio::process::Command::new(deepspeed_command);
        command
            .current_dir(&SERVER_CONFIG.fingerprinting_source_dir)
            .arg("--no_local_rank")
            .arg("generate_finetuning_data.py");

        for arg in payload.to_args() {
            command.arg(arg);
        }

        match command.spawn() {
            Ok(mut child) => {
                match child.wait().await {
                    Ok(exit_status) => {
                        println!("Generating fingerprints process exited with: {}", exit_status);
                        let mut app_state = state_for_thread.lock().unwrap();
                        app_state.generating_fingerprints = false;
                        app_state.config_hash = None;
                    }
                    Err(e) => {
                        eprintln!("Failed to wait on fingerprinting process: {}", e);
                        let mut app_state = state_for_thread.lock().unwrap();
                        app_state.generating_fingerprints = false;
                        app_state.config_hash = None;
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to spawn fingerprinting process: {}", e);
                let mut app_state = state_for_thread.lock().unwrap();
                app_state.generating_fingerprints = false;
                app_state.config_hash = None;
            }
        }
    });

    let response = json!({
        "status": "Started",
        "operation": "generate_fingerprints",
        "config_hash": hash,
    });
    Json(response)

   }
