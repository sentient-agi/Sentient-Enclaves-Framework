# Inference Server

An event-driven HTTP server for LLaMA model inference built in Rust using Axum and llamacpp-bindings.

## Features

### Core Features
- API endpoint for completions
- Local model inference
- Configurable model parameters
- Multi-threaded inference
- JSON response format

### Model Integration
- Local GGUF model loading
- Configurable inference parameters
- Real-time token generation
- Performance monitoring and metrics


## Setup

### Running the Server

1. Start the server:
```bash
cargo run --bin server
```

2. Test with the provided client:
```bash
cargo run --bin client
```

## Configuration

### Model Configuration
- Model path configuration
- Inference parameters
  - Max tokens
  - Context size
  - Thread count

### Server Configuration
- Default port: 3000
- Listening address: 0.0.0.0
- Customizable routes
