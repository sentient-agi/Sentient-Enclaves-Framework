use axum::{
    routing::{get, post},
    Router,
    Json,
    response::Response,
    response::IntoResponse,

};
use llamacpp_bindings::{LLM, ModelType, LoadParams, InferenceParams};
use std::path::PathBuf;
use std::sync::Arc;
use axum::extract::State;
use inference_server::{ CompletionRequest, LoadModelRequest };
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::num::NonZero;
use serde_json::json;
use clap::Command;

#[tokio::main]
async fn main() {
    let matches = Command::new("Inference Server")
        .version("1.0")
        .about("Runs the inference server")
        .arg(
            clap::Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the port to use")
                .required(false)
        )
        .get_matches();

    let default_port = "3000".to_string();
    let port = matches.get_one::<String>("port").unwrap_or(&default_port);

    let models: Arc<RwLock<HashMap<String, Arc<LLM>>>> = Arc::new(RwLock::new(HashMap::new()));
    
    let app = Router::new()
        .route("/", post(handle_post))
        .route("/completions", post(serve_completions))
        .route("/load_model", post(load_model_handler))
        .route("/status", get(status_handler))
        .with_state(models.clone());

    let address = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    println!("Server running at http://{}", address);
    axum::serve(listener, app).await.unwrap();
}

// async fn is_model_loaded(model_name: String, model_s) -> bool {}

async fn load_model_handler(
    State(models): State<Arc<RwLock<HashMap<String, Arc<LLM>>>>>,
    Json(payload): Json<LoadModelRequest>,
) -> Response {
    // Determine the source of the model
    let model_path = if payload.model_path.is_empty() {
        return IntoResponse::into_response("Error: Model path must be provided".to_string());
    } else {
        PathBuf::from(payload.model_path)
    };


    // Check if the model is already loaded
    {
        let models_lock = models.read().await;
        if models_lock.contains_key(&payload.model_name) {
            return IntoResponse::into_response(format!("Warning: Model {} already loaded", payload.model_name));
        }
    } // drop the lock

    // Load the model
    println!("Loading model: {}", payload.model_name);
    let model = LLM::load(ModelType::Local { path: model_path }, LoadParams::default()).unwrap();

    let mut models_lock = models.write().await;
    models_lock.insert(payload.model_name.clone(), Arc::new(model));
    
    Json(json!({
        "Message": format!("{} Model loaded", payload.model_name)
    })).into_response()

}

async fn status_handler(
    State(models): State<Arc<RwLock<HashMap<String, Arc<LLM>>>>>
) -> Response {
    let models_lock = models.read().await;
    let model_names: Vec<String> = models_lock.keys().cloned().collect();
    
    // Join the model names into a single string separated by newlines
    let response_text = model_names.join("\n");
    
        Json(json!({
            "Message": response_text
        })).into_response()
}


async fn serve_completions(State(models): State<Arc<RwLock<HashMap<String, Arc<LLM>>>>>, Json(payload): Json<CompletionRequest>) -> Response {
    let user_prompt = payload.prompt.clone();
    let prompt = format!("{}", user_prompt);
    let model_name = payload.model.clone();
    let max_tokens = payload.max_tokens;
    let inference_params = InferenceParams {
        max_tokens: max_tokens,
        seed: payload.seed,
        n_threads: Some(payload.n_threads),
        n_ctx: NonZero::new(payload.n_ctx).unwrap(),
        ..Default::default()
    };


    {
        let models_lock = models.read().await;
        if !models_lock.contains_key(&model_name) {
            return IntoResponse::into_response(format!("Error: Model {} not loaded. Please load the model using the /load_model endpoint.", model_name));
        }
    }

    let model = {
        let models_lock = models.read().await;
        models_lock.get(&model_name).cloned().unwrap()
    };

    let response = tokio::task::spawn_blocking(move || {
        let mut i = 1;
        println!("Prompt: {}", prompt);
        // Start a timer
        let start = std::time::Instant::now();
        let generated_text = model.predict(&prompt, inference_params, |token| {
            println!("{}: Token: {}", i, token);
            i += 1;
        }).unwrap();

        // End the timer
        let end = std::time::Instant::now();
        println!("Time taken: {:?}", end.duration_since(start));
        generated_text

    })
    .await
    .unwrap();

    // Convert the response struct to JSON
    Json(json!({
        "Message": response
    })).into_response()
}

async fn handle_post(Json(payload): Json<serde_json::Value>) -> Response {
    let prompt = payload["prompt"].as_str().unwrap();
    println!("Prompt: {}", prompt);
    IntoResponse::into_response("Hello, World returned!")
}