use reqwest::Client;
use std::error::Error;
use inference_server::{LoadModelRequest, CompletionRequest};
use rand::Rng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    
    let client = Client::new();

    while true {
        println!("================================================================================================");
        println!("Please choose an action: Load model, status, inference, or quit");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");
        let action = input.trim();
        println!("Action: {}", action);
        match action {
        "load_model" => {
            request_load_model(&client).await?;
        }
        "status" => {
            request_status(&client).await?;
        }
        "inference" => {
            println!("Please enter the prompt");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).expect("Failed to read line");
            let prompt = input.trim();
            request_inference(&client, prompt).await?;
        }
        "quit" => {
            break;
        }
        _ => {
            eprintln!("Unknown action: {}", action);
            eprintln!("Valid actions are: load_model, status, inference, or quit");
            }
        }
    }

    Ok(())
}

// 1. Request loading of the model
async fn request_load_model(client: &Client) -> Result<(), Box<dyn Error>> {
    let request_body = LoadModelRequest {
        model_name: "Dobby Unhinged".to_string(),
        model_path: "/home/ec2-user/pipeline/pipeline-tee.rs/reference_apps/inference_server/dobby-8b-unhinged-q4_k_m.gguf".to_string(),
    };

    let response = client
        .post("http://127.0.0.1:3000/load_model")
        .json(&request_body)
        .send()
        .await?;

    println!("Response: {:?}", response.text().await?);
    Ok(())
}

// 2. Request status using GET
async fn request_status(client: &Client) -> Result<(), Box<dyn Error>> {
    let response = client
        .get("http://127.0.0.1:3000/status")
        .send()
        .await?;

    println!("Response: {:?}", response.text().await?);

    Ok(())
}
async fn request_inference(client: &Client, prompt: &str) -> Result<(), Box<dyn Error>> {
    let mut rng = rand::thread_rng();
    let random_seed: u32 = rng.gen_range(1..=u32::MAX);

    let request_body = CompletionRequest {
        model: "LLaMa 3.1 8B".to_string(),
        prompt: prompt.to_string(),
        seed: random_seed,
        ..Default::default()
    };

    let response = client
        .post("http://127.0.0.1:3000/completions")
        .json(&request_body)
        .send()
        .await?;

    println!("Response: {:?}", response.text().await?);
    Ok(())
}