use reqwest::Client;
use std::error::Error;
use fingerprinting_server::{FingerprintRequest, GenerateFingerprintRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    
    let client = Client::new();

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
    let request_body = FingerprintRequest {
        model_path: "/home/ec2-user/oml-1.0-fingerprinting/meta_llama_3.1_8b_instruct_model".to_string(),
        num_fingerprints: 5,
        max_key_length: 16,
        max_response_length: 1,
        batch_size: 5,
        num_train_epochs: 10,
        learning_rate: 0.001,
        weight_decay: 0.0001,
        fingerprints_file_path: "/home/ec2-user/oml-1.0-fingerprinting/generated_data/output_fingerprints_demo.json".to_string(),
        fingerprint_generation_strategy: "english".to_string(),
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

    let request_body = GenerateFingerprintRequest {
        key_length: 16,
        response_length: 1,
        num_fingerprints: 5,
        batch_size: 5,
        model_used_for_key_generation: "meta_llama_3.1_8b_instruct_model".to_string(),
        key_response_strategy: "english".to_string(),
        output_file: "/home/ec2-user/oml-1.0-fingerprinting/generated_data/output_fingerprints_demo_new.json".to_string(),
        keys_file: "/home/ec2-user/oml-1.0-fingerprinting/generated_data/custom_fingerprints.json".to_string(),
    };

    let response = client
        .post("http://127.0.0.1:3000/generate_fingerprints")
        .json(&request_body)
        .send()
        .await?;

    handle_response(response).await?;
    Ok(())
}
