# Fingerprinting Server 🛡️🔍

An efficient and scalable HTTP server for generating and managing fingerprints built in Rust using Axum. Designed to handle fingerprint generation tasks seamlessly with configurable parameters and robust state management.

## Features 🌟

### Core Functionality
- **API Endpoints:**
  - `/fingerprint`: Initiates insertion of fingerprints into the model.
  - `/generate_fingerprints`: Generates multiple fingerprints based on provided parameters.
  - `/status`: Retrieves the current status of fingerprinting operations.
- **Concurrency:** Utilizes multi-threading to handle multiple fingerprinting tasks simultaneously.
- **State Management:** Maintains application state to monitor ongoing operations and configurations.
- **Configurable Parameters:** Customize fingerprinting operations with various parameters available in the [Sentient OML Fingerprinting](https://github.com/sentient-agi/oml-1.0-fingerprinting) library.
- **JSON Responses:** Consistent and structured JSON responses for all API interactions.

### Model Integration 🧠
- **Local Model Support:** Loads and manages local models for fingerprint generation.
- **Hash Generation:** Generates unique hashes based on fingerprint configurations to ensure consistency and traceability.
- **Error Handling:** Robust error management to handle process failures and command execution issues gracefully.

## Setup 🚀

### Prerequisites
- **Sentient OML Fingerprinting library:** Ensure you have the Sentient OML Fingerprinting repository cloned and it's dependencies installed. For more information, refer to the [Sentient OML Fingerprinting](https://github.com/sentient-agi/oml-1.0-fingerprinting) library. 
- **DeepSpeed:** Required for executing fingerprinting tasks. We recomment building it [from source](https://www.deepspeed.ai/tutorials/advanced-install/#install-deepspeed-from-source).

### Installation

1. **Configure the Server:**
    - Edit the `config.toml` file located in the `src` directory.
    - Set the `deepspeed_dir` to the path of the directory containing the DeepSpeed executable.
    - Set the `fingerprinting_source_dir` to the path of the directory containing the Sentient's OML Fingerprinting repository.
    - These can also be set via command-line arguments.

2. **Build the Server:**
    ```bash:fingerprinting_server/README.md
    cargo build --release
    ```
3. **Run the Server:** 🏁
    Start the fingerprinting server using Cargo:
    ```bash
    cargo run --bin fingerprinting_server
    ```
    The server will start and listen on the configured port (default: `3002`). You will see a message similar to:
    ```
    Server running at http://127.0.0.1:3002
    ```

## Configuration ⚙️

### Server Configuration
- **Port:** Default is `3001`. Can be changed in the `ServerConfig` within `lib.rs` or via command-line arguments.
- **DeepSpeed Directory:** Path to directory containing the DeepSpeed executable.
- **Fingerprinting Source Directory:** Path to directory containing the Sentient OML Fingerprinting repository.

### Model Configuration 🧠
- **Model Path:** Specify the path to your fingerprinting model.
- **Inference Parameters:**
  - **Max Key Length:** Maximum length of the generated key.
  - **Max Response Length:** Maximum length of the generated response.
  - **Batch Size:** Number of fingerprints to generate in a single batch.
  - **Number of Training Epochs:** Determines how many times the model will train on the data.
  - **Learning Rate & Weight Decay:** Hyperparameters for model training.
  - **Fingerprint Generation Strategy:** Strategy used for generating fingerprints (e.g., "english").

## API Endpoints 🖥️

### 1. Generate Fingerprint 🔐
**Endpoint:** `/fingerprint`  
**Method:** `POST`  
**Description:** Initiates the fingerprint generation process with the provided configuration.

**Request Body:**
```json:fingerprinting_server/src/lib.rs
{
    "model_path": "path/to/model",
    "num_fingerprints": 5,
    "max_key_length": 16,
    "max_response_length": 1,
    "batch_size": 5,
    "num_train_epochs": 10,
    "learning_rate": 0.001,
    "weight_decay": 0.0001,
    "fingerprints_file_path": "path/to/fingerprints.json",
    "fingerprint_generation_strategy": "english"
  },
```

**Response:**
```json:fingerprinting_server/src/lib.rs
{
  "status": "Started",
  "operation": "fingerprint",
  "config_hash": "unique_config_hash"
}
```

### 2. Insert Fingerprints into the model 🔐
**Endpoint:** `/generate_fingerprints`  
**Method:** `POST`  
**Description:** Generates multiple fingerprints based on the provided parameters.

**Request Body:**
```json:fingerprinting_server/src/lib.rs
{
  "key_length": 16,
  "response_length": 16,
  "num_fingerprints": 5,
  "batch_size": 5,
  "model_used_for_key_generation": "/path/to/model",
  "key_response_strategy": "independent",
  "output_file": "/path/to/output_fingerprints_demo_new.json"
}
```

**Response:**
```json:fingerprinting_server/src/lib.rs
{
  "status": "Started",
  "operation": "generate_fingerprints",
  "config_hash": "unique_config_hash"
}
```

### 3. Check Status 📊
**Endpoint:** `/status`  
**Method:** `GET`  
**Description:** Retrieves the current status of fingerprinting operations.

**Response:**
```json:fingerprinting_server/src/lib.rs
{
  "status": "In progress",
  "operation": "fingerprint",
  "config_hash": "unique_config_hash"
}
```
*Or*
```json:fingerprinting_server/src/lib.rs
{
  "status": "Available"
}
```

## Usage Example 📝

### Using the Provided Client

A simple client is provided to interact with the fingerprinting server. The `config.json` in the `tests` directory holds the configuration for fingerprint requests. This is used by the `client` to send requests to the server. Modify it as needed.


1. **Navigate to the Client Directory:**
    ```bash:fingerprinting_server/README.md
    cd tests
    ```

2. **Run the Client:**
    ```bash:fingerprinting_server/README.md
    cargo run --bin fingerprinting_client
    ```

3. **Choose an Action:**
    - **fingerprinting:** Initiates a fingerprinting task.
    - **status:** Checks the current status of the server.
    - **fingerprint_generation:** Generates multiple fingerprints.
    - **quit:** Exits the client.

### Sample `curl` Requests

**Start Fingerprinting:**
```bash:fingerprinting_server/README.md
curl -X POST http://127.0.0.1:3002/fingerprint \
-H "Content-Type: application/json" \
-d '{
  "model_path": "/path/to/model",
  "num_fingerprints": 5,
  "max_key_length": 16,
  "max_response_length": 1,
  "batch_size": 5,
  "num_train_epochs": 10,
  "learning_rate": 0.001,
  "weight_decay": 0.0001,
  "fingerprint_generation_strategy": "english",
  "fingerprints_file_path": "/path/to/output_fingerprints.json"
}'
```

**Check Status:**
```bash:fingerprinting_server/README.md
curl http://127.0.0.1:3002/status
```

**Generate Multiple Fingerprints:**
```bash:fingerprinting_server/README.md
curl -X POST http://127.0.0.1:3002/generate_fingerprints \
-H "Content-Type: application/json" \
-d '{
  "key_length": 16,
  "response_length": 16,
  "num_fingerprints": 5,
  "batch_size": 5,
  "model_used_for_key_generation": "/path/to/model",
  "key_response_strategy": "independent",
  "output_file": "/path/to/output_fingerprints_demo_new.json"
}'
```
