use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, )]
pub struct FingerprintRequest {
    pub model_path: String,
    pub num_fingerprints: usize,
    pub max_key_length: usize,
    pub max_response_length: usize,
    pub batch_size: usize,
    pub num_train_epochs: usize,
    pub learning_rate: f64,
    pub weight_decay: f64,
    pub fingerprint_generation_strategy: String,
    pub fingerprints_file_path: String,
}

impl FingerprintRequest {
    /// Converts the struct fields into a vector of command-line arguments.
    pub fn to_args(&self) -> Vec<String> {
        vec![
            "--model_path".to_string(),
            self.model_path.clone(),
            "--num_fingerprints".to_string(),
            self.num_fingerprints.to_string(),
            "--num_train_epochs".to_string(),
            self.num_train_epochs.to_string(),
            "--batch_size".to_string(),
            self.batch_size.to_string(),
            "--fingerprints_file_path".to_string(),
            self.fingerprints_file_path.clone(),
            "--fingerprint_generation_strategy".to_string(),
            self.fingerprint_generation_strategy.clone(),
        ]
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GenerateFingerprintRequest {
    pub key_length: u32,
    pub response_length: u32,
    pub num_fingerprints: u32,
    pub batch_size: u32,
    pub model_used_for_key_generation: String,
    pub key_response_strategy: String,
    pub output_file: String,
    pub keys_file: String,
}

impl GenerateFingerprintRequest {
    pub fn to_args(&self) -> Vec<String> {
        vec![
            "--key_length".to_string(),
            self.key_length.to_string(),
            "--response_length".to_string(),
            self.response_length.to_string(),
            "--num_fingerprints".to_string(),
            self.num_fingerprints.to_string(),
            "--batch_size".to_string(),
            self.batch_size.to_string(),
            "--model_used_for_key_generation".to_string(),
            self.model_used_for_key_generation.clone(),
            "--key_response_strategy".to_string(),
            self.key_response_strategy.clone(),
            "--output_file".to_string(),
            self.output_file.clone(),
            "--keys_file".to_string(),
            self.keys_file.clone(),
        ]
    }
}
