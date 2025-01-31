use serde::{Deserialize, Serialize};
use clap::Parser;
use std::path::PathBuf;
use config;

#[derive(Debug, Serialize, Deserialize, )]
pub struct FingerprintRequest {
    pub model_path: String,
    pub num_fingerprints: u32,
    pub max_key_length: u32,
    pub max_response_length: u32,
    pub batch_size: u32,
    pub num_train_epochs: u32,
    pub learning_rate: f32,
    pub weight_decay: f32,
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
        ]
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(long, help = "Path to config file")]
    pub config: Option<PathBuf>,
    
    #[arg(long, help = "Path to deepspeed executable")]
    pub deepspeed_path: Option<String>,
    
    #[arg(long, help = "Working directory for fingerprinting operations")]
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub deepspeed_dir: String,
    pub fingerprinting_source_dir: String,
}

impl ServerConfig {
    pub fn new() -> Result<Self, config::ConfigError> {
        let args = Args::parse();
        
        // Start with default config
        let mut settings = config::Config::builder()
            .set_default("port", 3001)?
            .set_default("deepspeed_dir", std::env::current_dir().unwrap().to_str().unwrap())?
            .set_default("fingerprinting_source_dir", std::env::current_dir().unwrap().to_str().unwrap())?;

        // Add config file if specified
        if let Some(config_path) = args.config {
            settings = settings.add_source(config::File::from(config_path));
        } else {
            // Try at src/config.toml and src/config.local.toml
            settings = settings
                .add_source(config::File::with_name("config").required(false))
                .add_source(config::File::with_name("config.local").required(false));
        }

        let config = settings.build()?;

        let deepspeed_dir = args.deepspeed_path
            .unwrap_or_else(|| config.get_string("deepspeed_dir").unwrap());
        let fingerprinting_source_dir = args.working_dir
            .unwrap_or_else(|| config.get_string("fingerprinting_source_dir").unwrap());
        let port = config.get_int("port").unwrap() as u16;
        
        println!("deepspeed_dir: {}", deepspeed_dir);
        println!("fingerprinting_source_dir: {}", fingerprinting_source_dir);
        println!("port: {}", port);
        Ok(ServerConfig {
            port,
            deepspeed_dir,
            fingerprinting_source_dir,
        })
    }
}

