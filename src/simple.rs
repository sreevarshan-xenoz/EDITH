use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "llm")]
#[command(about = "Simple LLM wrapper")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Model to use
    #[arg(short, long, default_value = "llama3.2")]
    model: String,
    
    /// Base URL for LLM server
    #[arg(short, long, default_value = "http://localhost:11434")]
    url: String,
    
    /// Single message mode
    message: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive chat mode
    Chat,
    /// List available models
    List,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    response: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::List) => {
            println!("Available models:");
            println!("  - llama3.2");
            println!("  - llava");
            println!("  - Run 'ollama list' for full list");
        }
        Some(Commands::Chat) => {
            interactive_mode(&cli.model, &cli.url)?;
        }
        None => {
            if let Some(message) = cli.message {
                // Single message mode
                let response = send_message(&cli.model, &cli.url, &message)?;
                println!("{}", response);
            } else {
                // Interactive mode
                interactive_mode(&cli.model, &cli.url)?;
            }
        }
    }
    
    Ok(())
}

fn interactive_mode(model: &str, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Connected to: {}", model);
    println!("Commands: /quit to exit");
    println!("{}", "-".repeat(50));
    
    loop {
        print!("💬 You: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        if input == "/quit" || input == "/q" {
            break;
        }
        
        print!("🤖 Assistant: ");
        io::stdout().flush()?;
        
        match send_message(model, base_url, input) {
            Ok(response) => {
                println!("{}", response);
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
    
    Ok(())
}

fn send_message(model: &str, base_url: &str, message: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Use curl as a fallback since it's likely installed
    let request = ChatRequest {
        model: model.to_string(),
        prompt: message.to_string(),
        stream: false,
    };
    
    let json_data = serde_json::to_string(&request)?;
    let url = format!("{}/api/generate", base_url);
    
    let output = Command::new("curl")
        .arg("-X")
        .arg("POST")
        .arg(&url)
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-d")
        .arg(&json_data)
        .arg("--silent")
        .output()?;
    
    if output.status.success() {
        let response_text = String::from_utf8(output.stdout)?;
        let response: ChatResponse = serde_json::from_str(&response_text)?;
        Ok(response.response)
    } else {
        let error = String::from_utf8(output.stderr)?;
        Err(format!("Request failed: {}", error).into())
    }
}