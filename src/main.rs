use clap::{Parser, Subcommand};
use llm_wrapper::{LLMWrapper, Config};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "llm")]
#[command(about = "Universal local LLM wrapper with auto-capability detection")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Model to use
    #[arg(short, long, default_value = "llama3.2")]
    model: String,
    
    /// Base URL for LLM server
    #[arg(short, long, default_value = "http://localhost:11434")]
    url: String,
    
    /// System prompt
    #[arg(short, long)]
    system: Option<String>,
    
    /// Image files to include
    #[arg(short, long)]
    image: Vec<PathBuf>,
    
    /// Single message mode
    message: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// List available models
    List,
    /// Pull a model
    Pull { model: String },
    /// Delete a model
    Delete { model: String },
    /// Interactive chat mode
    Chat,
    /// Show model capabilities
    Info { model: Option<String> },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let config = Config::load("config.toml").unwrap_or_default();
    let mut wrapper = LLMWrapper::new(&cli.url, &cli.model, config).await?;
    
    match cli.command {
        Some(Commands::List) => {
            let models = wrapper.list_models().await?;
            println!("Available models:");
            for model in models {
                println!("  - {}", model);
            }
        }
        Some(Commands::Pull { model }) => {
            wrapper.pull_model(&model).await?;
        }
        Some(Commands::Delete { model }) => {
            wrapper.delete_model(&model).await?;
        }
        Some(Commands::Chat) => {
            interactive_mode(wrapper, cli.model.clone()).await?;
        }
        Some(Commands::Info { model }) => {
            let model_name = model.as_deref().unwrap_or(&cli.model);
            wrapper.switch_model(model_name).await?;
            let caps = wrapper.capabilities();
            println!("Model: {}", model_name);
            println!("Vision: {}", if caps.supports_vision { "✅" } else { "❌" });
            println!("Thinking: {}", if caps.supports_thinking { "✅" } else { "❌" });
            println!("Streaming: {}", if caps.supports_streaming { "✅" } else { "❌" });
        }
        None => {
            if let Some(message) = cli.message {
                // Single message mode
                let response = wrapper.chat(&message, &cli.image, cli.system.as_deref()).await?;
                println!("{}", response);
            } else {
                // Interactive mode
                interactive_mode(wrapper, cli.model.clone()).await?;
            }
        }
    }
    
    Ok(())
}

async fn interactive_mode(mut wrapper: LLMWrapper, model_name: String) -> anyhow::Result<()> {
    use std::io::{self, Write};
    
    let caps = wrapper.capabilities();
    println!("🤖 Connected to: {}", model_name);
    println!("📷 Vision: {} | 🧠 Thinking: {} | 💬 Streaming: {}", 
        if caps.supports_vision { "✅" } else { "❌" },
        if caps.supports_thinking { "✅" } else { "❌" },
        if caps.supports_streaming { "✅" } else { "❌" }
    );
    println!("Commands: /image <path>, /model <name>, /clear, /quit");
    println!("{}", "-".repeat(50));
    
    let mut current_images: Vec<PathBuf> = Vec::new();
    
    loop {
        print!("💬 You: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        // Handle commands
        if input.starts_with('/') {
            let parts: Vec<&str> = input.splitn(2, ' ').collect();
            match parts[0] {
                "/quit" | "/q" => break,
                "/image" => {
                    if parts.len() > 1 {
                        let path = PathBuf::from(parts[1]);
                        if path.exists() {
                            current_images.push(path.clone());
                            println!("📷 Added: {}", path.display());
                        } else {
                            println!("❌ File not found: {}", parts[1]);
                        }
                    }
                }
                "/model" => {
                    if parts.len() > 1 {
                        match wrapper.switch_model(parts[1]).await {
                            Ok(_) => {
                                let caps = wrapper.capabilities();
                                println!("✅ Switched to: {}", parts[1]);
                            }
                            Err(e) => println!("❌ Error: {}", e),
                        }
                    }
                }
                "/clear" => {
                    current_images.clear();
                    println!("🗑️ Cleared images");
                }
                _ => println!("❌ Unknown command: {}", parts[0]),
            }
        } else {
            // Send message
            print!("🤖 Assistant: ");
            io::stdout().flush()?;
            
            match wrapper.chat(input, &current_images, None).await {
                Ok(response) => {
                    println!("{}", response);
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                }
            }
            
            current_images.clear();
        }
    }
    
    Ok(())
}