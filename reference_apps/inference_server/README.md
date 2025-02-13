# Inference Server 🚀

An event-driven, light-weight HTTP server for model inference built in Rust using Axum and llamacpp-bindings. This server is primarily written to be used as a standlone binary to be run inside TEEs using Sentient's Secure Enclave Framework.

## Features ⭐

### Core
- **API Endpoint:** Supports completions.
- **Local Inference:** Runs models locally.
- **Configurable Parameters:** Adjust model settings as needed.
- **Multi-threaded:** Enhances performance with multi-threading.
- **JSON Responses:** Standardized response format.

### Supported Endpoints 📡
All endpoints by default are available at `http://0.0.0.0:3000`. A `curl` request can be made to following endpoints to use the server.
#### `/completions` 
- Supports subset of OpenAI API completions endpoint.
- A `POST` request to this endpoint will perform inference on the model specified in the request body.
- The request format is as follows:
  ```rust
  pub struct CompletionRequest {
    pub model: String,
    pub prompt: String,
    pub max_tokens: i32,
    pub seed: u32,
    pub n_threads: i32,
    pub n_ctx: u32,
  }
  ```

#### `/load_model`
> [!NOTE]
> The model must be present in GGUF format. If the model is not in GGUF format, convert it using [model_converter](../model_converter/).
- A `POST` request to this endpoint will load a model into the server.
- The model is loaded into the server's memory and can be used for inference.
- The request format is as follows:
  ```rust
  pub struct LoadModelRequest {
    pub model_name: String,
    pub model_path: String,
  }
  ```

#### `/status`
- A `GET` request to this endpoint enumerates all the models loaded into the server.


### Model Integration 🧠
- **GGUF Support:** Loads local GGUF models.
- **Flexible Inference:** Customize parameters like max tokens and context size.
- **Real-time Token Generation:** Provides instant token outputs.

## Setup 🛠️

### Running the Server
1. **Start the Server:**
    ```bash
    cargo run --bin inference_server
    ```
## Configuration ⚙️

### Model Configuration
- **Model Path:** Specify the path to your model.
- **Inference Parameters:**
  - Max tokens
  - Context size
  - Thread count

### Server Configuration
- **Port:** Default is `3000`.
- **Address:** Listens on `0.0.0.0`.