use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CompletionRequest {
    pub model: String,
    pub prompt: String,
    pub max_tokens: i32,
    pub seed: u32,
    pub n_threads: i32,
    pub n_ctx: u32,
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            model: "".to_string(),
            prompt: "Who are you?".to_string(),
            max_tokens: 100,      
            seed: 1234,
            n_threads: 5,
            n_ctx: 2048,
        }
    }
}
// #[derive(Serialize, Deserialize, Debug)]
// pub struct CompletionChoice {
//     pub text: String,
//     pub index: u32,
//     pub logprobs: Option<serde_json::Value>,
//     pub finish_reason: Option<String>,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct CompletionResponse {
//     pub id: String,
//     pub object: String,
//     pub created: u64,
//     pub model: String,
//     pub choices: Vec<CompletionChoice>,
// }

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadModelRequest {
    pub model_name: String,
    pub model_path: Option<String>,
}