use anyhow::{ Context, Result};
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::{LlamaModel, params::LlamaModelParams, Special, AddBos};
use llama_cpp_2::token::data_array::LlamaTokenDataArray;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_batch::LlamaBatch;
use std::num::NonZeroU32;
use std::path::PathBuf;
use encoding_rs::UTF_8;
use hf_hub::api::sync::ApiBuilder;
use llama_cpp_2::ggml_time_us;
use std::time::Duration;
use once_cell::sync::OnceCell;

static BACKEND: OnceCell<LlamaBackend> = OnceCell::new();

// Enum for selecting model type
#[derive(Debug, Clone)]
pub enum ModelType {
    Local { path: PathBuf },
    // HuggingFace { repo: String, model: String },
}

/// High-level handle for model parameters
#[derive(Debug, Clone)]
pub struct LoadParams {
    pub n_gpu_layers: u32,
    pub use_mmap: bool,
    pub use_mlock: bool,
    pub vocab_only: bool,
    // pub kv_overrides: Vec<(String, ParamOverrideValue)>,
}

impl Default for LoadParams {
    fn default() -> Self {
        Self {
            n_gpu_layers: 0,
            use_mmap: true,
            use_mlock: false,
            vocab_only: false,
            // kv_overrides: Vec::new(),
        }
    }
}

/// High-level handle for context parameters
#[derive(Debug, Clone)]
pub struct InferenceParams {
    pub n_ctx: NonZeroU32,
    pub n_threads: Option<i32>,
    pub n_threads_batch: Option<i32>,
    pub seed: u32,
    pub max_tokens: i32,
    // pub temperature: f32, No support for temperature in llama_cpp_2
    pub embeddings: bool,
}

impl Default for InferenceParams {
    fn default() -> Self {
        Self {
            n_ctx: NonZeroU32::new(2048).unwrap(),
            n_threads: None,
            n_threads_batch: None,
            seed: 1234,
            max_tokens: 100,
            embeddings: false,
        }
    }
}

// Internal conversion traits (private to crate)
impl From<LoadParams> for llama_cpp_2::model::params::LlamaModelParams {
    fn from(params: LoadParams) -> Self {
        // Initialize default parameters and set existing fields
        let mut model_params = llama_cpp_2::model::params::LlamaModelParams::default()
            .with_n_gpu_layers(params.n_gpu_layers as u32)
            .with_use_mlock(params.use_mlock)
            .with_vocab_only(params.vocab_only);
        model_params
    }        
}

impl From<InferenceParams> for llama_cpp_2::context::params::LlamaContextParams {
    fn from(params: InferenceParams) -> Self {
        let mut ctx_params = Self::default().with_n_ctx(Some(params.n_ctx)).with_seed(params.seed);
        if let Some(n_threads) = params.n_threads {
            ctx_params = ctx_params.with_n_threads(n_threads);
        }
        if let Some(n_threads_batch) = params.n_threads_batch {
            ctx_params = ctx_params.with_n_threads_batch(n_threads_batch);
        }
        ctx_params = ctx_params.with_embeddings(params.embeddings);
        ctx_params
    }
}
// Struct representing the Language Model
pub struct LLM {
    model: LlamaModel,
}

impl LLM {
    /// Loads the model based on the provided ModelType
    pub fn load(model_type: ModelType, load_params: LoadParams) -> Result<Self> {
        // Initialize backend only once
        let backend = BACKEND.get_or_try_init(LlamaBackend::init)?;

        let model_path = match model_type {
            ModelType::Local { path } => path
        };
        
        let model_params = LlamaModelParams::from(load_params);
        let model = LlamaModel::load_from_file(backend, &model_path, &model_params)
            .with_context(|| "unable to load model")?;
            
        Ok(LLM {
            model,
        })
    }

    /// Performs prediction based on the prompt and current parameters
    pub fn predict<F>(&self, prompt: &str, inference_params: InferenceParams, mut callback: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        let backend = BACKEND.get().expect("Backend not initialized");
        let t_main_start = ggml_time_us();
        let max_tokens = inference_params.max_tokens;
        let ctx_params = LlamaContextParams::from(inference_params);
        
        let mut ctx = self
            .model
            .new_context(backend, ctx_params)
            .context("unable to create the llama_context")?;

        // Tokenize the prompt
        let tokens_list = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .context("failed to tokenize prompt")?;

        // Create batch for token processing
        let mut batch = LlamaBatch::new(512, 1);
        let last_index: i32 = (tokens_list.len() - 1) as i32;

        for (i, token) in (0_i32..).zip(tokens_list.into_iter()) {
            batch.add(token, i, &[0], i == last_index)?;
        }

        ctx.decode(&mut batch)?;

        // Generation loop
        let mut n_cur = batch.n_tokens();
        let mut output = String::new();
        let mut decoder = UTF_8.new_decoder();

        let mut n_decode = 0;
        while n_cur <= max_tokens {
            let candidates = ctx.candidates();
            let candidates_p = LlamaTokenDataArray::from_iter(candidates, false);
            let new_token_id = ctx.sample_token_greedy(candidates_p);

            // Check for end of generation
            if self.model.is_eog_token(new_token_id) {
                break;
            }

            // Decode token to string
            let output_bytes = self.model.token_to_bytes(new_token_id, Special::Tokenize)?;
            let mut token_string = String::with_capacity(32);
            decoder.decode_to_string(&output_bytes, &mut token_string, false);
            output.push_str(&token_string);

            // Call the callback with the new token
            callback(&token_string);
            n_decode += 1;
            // Process next token
            batch.clear();
            batch.add(new_token_id, n_cur, &[0], true)?;
            ctx.decode(&mut batch)?;
            n_cur += 1;
        }

        let t_main_end = ggml_time_us();

        let duration = Duration::from_micros((t_main_end - t_main_start) as u64);

        eprintln!(
            "Time: {:.2}s, Speed: {:.2} t/s\n",
            duration.as_secs_f32(),
            n_decode as f32 / duration.as_secs_f32()
        );
    
        callback("\n");

        Ok(output)
    }
}
