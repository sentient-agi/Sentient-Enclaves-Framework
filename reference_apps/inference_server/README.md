# Inference Server 🚀

An event-driven, light-weight HTTP server for model inference built in Rust using Axum and llamacpp-bindings. This server is primarily written to be used as a standalone binary to be run inside TEEs using Sentient's Secure Enclave Framework.

## Features ⭐

### Core ⚙️
- **API Endpoint:** Supports a set of endpoints for model inference.
- **Local Inference:** Runs model inference locally.
- **Model Management:** Allows for concurrent loading of multiple models into the server.
- **Configurable Parameters:** Adjust model settings on per model, per request basis.
- **Multi-threaded:** Utilizes multi-threading for processing each inference request.

### Supported Endpoints 📡
All endpoints by default are available at `http://127.0.0.1:3000`. 

A simple `curl` request can be made to the following endpoints to use the server.

#### `/completions` ✍️
- Supports a subset of the OpenAI API completions endpoint.
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

#### `/load_model` 🗂️
> [!NOTE]
> The model must be present in GGUF format. If the model is not in GGUF format, convert it using [model_converter](../model_converter/).
- A `POST` request to this endpoint will load a model into the server.
- The model is loaded into the server's memory and can be used for inference.
- - The request format is as follows:
  ```rust
  pub struct LoadModelRequest {
    pub model_name: String,
    pub model_path: String,
  }
  ```

#### `/status` 📊
- A `GET` request to this endpoint enumerates all the models loaded into the server.

### Model Integration 🧠
- **GGUF Support:** Loads local GGUF models.
- **Flexible Inference:** Customize parameters like max tokens and context size.
- **Real-time Token Generation:** Provides instant token outputs.

## Setup 🛠️

### Running the Server 🚀
An example of how to write a client to interact with the server is provided in [tests/client.rs](../tests/client.rs).
1. **Start the Server:**
    ```bash
    cargo run --bin inference_server
    ```
2. **Load a Model:**
    ```bash
    curl -X POST http://127.0.0.1:3000/load_model -H "Content-Type: application/json" -d '{"model_name": "llama3-8b", "model_path": "/path/to/llama3-8b.gguf"}'
    ```
3. **Perform Inference:**
    ```bash
    curl -X POST http://127.0.0.1:3000/completions -H "Content-Type: application/json" -d '{"model": "llama3-8b", "prompt": "Hello, world!", "max_tokens": 10, "seed": 42, "n_threads": 4, "n_ctx": 512}'
    ```
4. **Get Status:**
    ```bash
    curl http://127.0.0.1:3000/status
    ```

## Configuration ⚙️

### Model Configuration 🧠
- **Model Path:** Specify the path to your model.
- **Inference Parameters:**
  - Max tokens
  - Context size
  - Thread count
  - Seed
  - Context Window

### Server Configuration 🌍
- **Port:** Default port is `3000`. To change the port, use the `-p` flag.
- **Address:** Listens on `127.0.0.1`.
