# LLaMa CPP Bindings

Rust bindings for llama.cpp providing high-performance inference capabilities for LLaMA models. These provide a high-level interface for loading and using LLaMA models built on top of utilityai/llama-cpp-rs.

## Features

### Model Support
- Local model loading from GGUF format
- Support for multiple model architectures
- Configurable model parameters and KV cache overrides


### Inference Capabilities
- Token-by-token generation
- Callback support for handling generated tokens as they are produced
- Configurable context window (default: 2048 tokens)
- Adjustable batch sizes
- Performance metrics tracking

### Features
- Multi-threading support for both inference and batch processing
- Configurable seed for reproducible results
- Performance monitoring with tokens/second metrics
- Memory-efficient token handling

## Usage

### Basic Example
```rust
use llamacpp_bindings::{LLM, ModelType, InferenceParams};

// Load a local model
let model = LLM::load(
    ModelType::Local { 
        path: "path/to/model.gguf".into() 
    },
    vec![]
)?;

// Generate text
let output = model.predict("Your prompt here", |token| {
    println!("Generated token: {}", token);
})?;
```

### Custom Inference Parameters
```rust
use std::num::NonZeroU32;

let params = InferenceParams {
    max_tokens: 100,
    seed: 1234,
    n_ctx: NonZeroU32::new(2048).unwrap(),
    n_threads: Some(4),
    n_threads_batch: Some(4),
    key_value_overrides: vec![],
};

model.set_params(params);
```

## API Reference

### Core Types
- `LLM`: Main model interface
- `ModelType`: Model loading configuration
- `InferenceParams`: Generation parameters

For implementation details see:

`llamacpp_bindings/src/lib.rs`

```rust
// Enum for selecting model type
#[derive(Debug, Clone)]
pub enum ModelType {
    Local { path: PathBuf },
    HuggingFace { repo: String, model: String },
}

// Struct to hold inference parameters
#[derive(Debug, Clone)]
pub struct InferenceParams {
    pub max_tokens: i32,
    pub seed: u32,
    pub n_ctx: NonZeroU32,
    pub n_threads: Option<i32>,
    pub n_threads_batch: Option<i32>,
    pub key_value_overrides: Vec<(String, ParamOverrideValue)>,
}

impl Default for InferenceParams {
    fn default() -> Self {
        InferenceParams {
            max_tokens: 100,
            seed: 1234,
            n_ctx: NonZeroU32::new(2048).unwrap(),
            // Add n_batch, logits_all, 
            n_threads: None,
            n_threads_batch: None,
            key_value_overrides: Vec::new(),
        }
    }
}

// Struct representing the Language Model
pub struct LLM {
    backend: LlamaBackend,
    model: LlamaModel,
    params: InferenceParams,
}
```

