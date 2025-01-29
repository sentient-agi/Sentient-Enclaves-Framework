use reqwest::Client;
use std::error::Error;
use fingerprinting_server::{FingerprintRequest, GenerateFingerprintRequest};
use std::fs;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    
    let client = Client::new();
    let config = fs::read_to_string("fingerprinting_server/tests/config.json")?;
    let config: Value = serde_json::from_str(&config)?;

    while true {
        println!("================================================================================================");
        println!("Please choose an action: fingerprinting, status, or fingerprint_generation, or quit");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");
        let action = input.trim();
        println!("Action: {}", action);
        match action {
        "fingerprinting" => {
            request_finetuning(&client).await?;
        }
        "status" => {
            request_status(&client).await?;
        }
        "fingerprint_generation" => {
            request_fingerprint_generation(&client).await?;
        }
        "quit" => {
            break;
        }
        _ => {
            eprintln!("Unknown action: {}", action);
            eprintln!("Valid actions are: fingerprinting, status, fingerprint_generation, or quit");
            }
        }
    }

    Ok(())
}

async fn handle_response(response: reqwest::Response) -> Result<(), Box<dyn Error>> {
    if response.status().is_success() {
        let status: serde_json::Value = response.json().await?;
        if status.get("status").unwrap() == "In progress" {
            if status.get("operation").unwrap() == "fingerprint" {
                println!("A fingerprinting job is already running with the following config hash: {:?}", status.get("config_hash").unwrap());
            } else if status.get("operation").unwrap() == "generate_fingerprints" {
                println!("A fingerprints generation job is already running with the following config hash: {:?}", status.get("config_hash").unwrap());
            }
            
        } 
        else if status.get("status").unwrap() == "Started" {
            if status.get("operation").unwrap() == "fingerprint" {
                println!("A fingerprinting job is started with the following config hash: {:?}", status.get("config_hash").unwrap());
            } else if status.get("operation").unwrap() == "generate_fingerprints" {
                println!("A fingerprints generation job is started with the following config hash: {:?}", status.get("config_hash").unwrap());
            }
        } else {
            println!("Server is available to accept a new fingerprinting or fingerprints generation job!");
        }
    } else {
        eprintln!("Status request failed with status: {}", response.status());
    }
    Ok(())
}

// 1. Request fingerprinting using POST 
async fn request_finetuning(client: &Client) -> Result<(), Box<dyn Error>> {
    let config = fs::read_to_string("fingerprinting_server/tests/config.json")?;
    let config: Value = serde_json::from_str(&config)?;
    
    let request_body = FingerprintRequest {
        model_path: config["fingerprint_request"]["model_path"]
            .as_str()
            .ok_or("model_path missing or invalid")?
            .to_string(),
        num_fingerprints: config["fingerprint_request"]["num_fingerprints"]
            .as_u64()
            .ok_or("num_fingerprints missing or invalid")?
            as u32,
        max_key_length: config["fingerprint_request"]["max_key_length"]
            .as_u64()
            .ok_or("max_key_length missing or invalid")?
            as u32,
        max_response_length: config["fingerprint_request"]["max_response_length"]
            .as_u64()
            .ok_or("max_response_length missing or invalid")?
            as u32,
        batch_size: config["fingerprint_request"]["batch_size"]
            .as_u64()
            .ok_or("batch_size missing or invalid")?
            as u32,
        num_train_epochs: config["fingerprint_request"]["num_train_epochs"]
            .as_u64()
            .ok_or("num_train_epochs missing or invalid")?
            as u32,
        learning_rate: config["fingerprint_request"]["learning_rate"]
            .as_f64()
            .ok_or("learning_rate missing or invalid")?
            as f32,
        weight_decay: config["fingerprint_request"]["weight_decay"]
            .as_f64()
            .ok_or("weight_decay missing or invalid")?
            as f32,
        fingerprints_file_path: config["fingerprint_request"]["fingerprints_file_path"]
            .as_str()
            .ok_or("fingerprints_file_path missing or invalid")?
            .to_string(),
        fingerprint_generation_strategy: config["fingerprint_request"]["fingerprint_generation_strategy"]
            .as_str()
            .ok_or("fingerprint_generation_strategy missing or invalid")?
            .to_string(),
    };

    let response = client
        .post("http://127.0.0.1:3000/fingerprint")
        .json(&request_body)
        .send()
        .await?;

    handle_response(response).await?;

    Ok(())
}



// 2. Request status using GET
async fn request_status(client: &Client) -> Result<(), Box<dyn Error>> {
    let response = client
        .get("http://127.0.0.1:3000/status")
        .send()
        .await?;

    handle_response(response).await?;

    Ok(())
}
async fn request_fingerprint_generation(client: &Client) -> Result<(), Box<dyn Error>> {
    let config = fs::read_to_string("fingerprinting_server/tests/config.json")?;
    let config: Value = serde_json::from_str(&config)?;

    let request_body = GenerateFingerprintRequest {
        key_length: config["generate_fingerprint_request"]["key_length"]
            .as_u64()
            .ok_or("key_length missing or invalid")?
            as u32,
        response_length: config["generate_fingerprint_request"]["response_length"]
            .as_u64()
            .ok_or("response_length missing or invalid")?
            as u32,
        num_fingerprints: config["generate_fingerprint_request"]["num_fingerprints"]
            .as_u64()
            .ok_or("num_fingerprints missing or invalid")?
            as u32,
        batch_size: config["generate_fingerprint_request"]["batch_size"]
            .as_u64()
            .ok_or("batch_size missing or invalid")?
            as u32,
        model_used_for_key_generation: config["generate_fingerprint_request"]["model_used_for_key_generation"]
            .as_str()
            .ok_or("model_used_for_key_generation missing or invalid")?
            .to_string(),
        key_response_strategy: config["generate_fingerprint_request"]["key_response_strategy"]
            .as_str()
            .ok_or("key_response_strategy missing or invalid")?
            .to_string(),
        output_file: config["generate_fingerprint_request"]["output_file"]
            .as_str()
            .ok_or("output_file missing or invalid")?
            .to_string(),
    };

    let response = client
        .post("http://127.0.0.1:3000/generate_fingerprints")
        .json(&request_body)
        .send()
        .await?;

    handle_response(response).await?;
    Ok(())
}
