use clap::{Parser, Subcommand};
use llm_wrapper::{LLMWrapper, Config, EnhancedLLMWrapper, EnhancedConfig, Template};
use std::path::PathBuf;
use serde_json::json;

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
    /// Interactive chat mode with enhanced TUI
    Chat,
    /// Show model capabilities
    Info { model: Option<String> },
    /// Enhanced mode with all features
    Enhanced {
        #[command(subcommand)]
        command: Option<EnhancedCommands>,
    },
}

#[derive(Subcommand)]
enum EnhancedCommands {
    /// Interactive mode with TUI
    Interactive,
    /// Template management
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
    /// Cache management
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Chat with template
    ChatTemplate {
        /// Template name
        template: String,
        /// Variables as JSON
        #[arg(short, long)]
        vars: Option<String>,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Show metrics and statistics
    Stats,
}

#[derive(Subcommand)]
enum TemplateAction {
    /// List available templates
    List,
    /// Create a new template
    Create {
        /// Template name
        name: String,
        /// Template content file
        file: PathBuf,
        /// Template description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Show template details
    Show { name: String },
    /// Delete a template
    Delete { name: String },
}

#[derive(Subcommand)]
enum CacheAction {
    /// Show cache statistics
    Stats,
    /// Clear all cache
    Clear,
    /// Clear cache for specific model
    ClearModel { model: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Enhanced { command }) => {
            // Use enhanced wrapper with all features
            let enhanced_config = load_enhanced_config().await?;
            let mut enhanced_wrapper = EnhancedLLMWrapper::new(enhanced_config).await?;
            
            match command {
                Some(EnhancedCommands::Interactive) => {
                    enhanced_wrapper.interactive_mode().await?;
                }
                Some(EnhancedCommands::Template { action }) => {
                    handle_template_command(&mut enhanced_wrapper, action).await?;
                }
                Some(EnhancedCommands::Cache { action }) => {
                    handle_cache_command(&mut enhanced_wrapper, action).await?;
                }
                Some(EnhancedCommands::ChatTemplate { template, vars, model }) => {
                    let variables = if let Some(vars_str) = vars {
                        serde_json::from_str(&vars_str)?
                    } else {
                        json!({})
                    };
                    
                    let stream_response = enhanced_wrapper.chat_with_template(&template, variables, model.as_deref()).await?;
                    println!("🤖 Response (streaming):");
                    // For CLI, we'll just collect the stream and print it
                    // In a real implementation, you'd want to handle the stream properly
                    println!("Stream created with ID: {}", stream_response.id);
                }
                Some(EnhancedCommands::Stats) => {
                    let metrics = enhanced_wrapper.get_metrics();
                    let cache_stats = enhanced_wrapper.get_cache_stats();
                    
                    println!("📊 Enhanced LLM Wrapper Statistics");
                    println!("═══════════════════════════════════");
                    println!("🔢 Total Requests: {}", metrics.requests_total);
                    println!("⚡ Average Response Time: {:.2}ms", metrics.average_response_time_ms);
                    println!("📋 Cache Hit Ratio: {:.1}%", metrics.cache_hit_ratio() * 100.0);
                    println!("🎯 Cache Hits: {}", metrics.cache_hits);
                    println!("❌ Cache Misses: {}", metrics.cache_misses);
                    println!("📝 Template Renders: {}", metrics.template_renders);
                    println!("🌊 Active Streams: {}", metrics.active_streams);
                    println!("⚠️  Total Errors: {}", metrics.errors_total);
                    println!();
                    println!("💾 Cache Details:");
                    println!("  Total Entries: {}", cache_stats.total_entries);
                    println!("  Memory Usage: {} bytes", cache_stats.memory_usage_bytes);
                    println!("  Evictions: {}", cache_stats.evictions);
                    println!("  Disk Reads: {}", cache_stats.disk_reads);
                    println!("  Disk Writes: {}", cache_stats.disk_writes);
                }
                None => {
                    // Default to interactive mode
                    enhanced_wrapper.interactive_mode().await?;
                }
            }
        }
        _ => {
            // Legacy mode - use original wrapper
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
                _ => unreachable!(),
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
                                let _caps = wrapper.capabilities();
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

async fn load_enhanced_config() -> anyhow::Result<EnhancedConfig> {
    // Try to load from enhanced-config.toml, fall back to defaults
    match EnhancedConfig::load("enhanced-config.toml") {
        Ok(config) => {
            println!("✅ Loaded configuration from enhanced-config.toml");
            Ok(config)
        }
        Err(e) => {
            println!("⚠️  Failed to load enhanced-config.toml: {}", e);
            println!("ℹ️  Using default configuration");
            let default_config = EnhancedConfig::default();
            
            // Save default config for future reference
            if let Err(save_err) = default_config.save("enhanced-config.toml") {
                println!("⚠️  Failed to save default config: {}", save_err);
            } else {
                println!("💾 Saved default configuration to enhanced-config.toml");
            }
            
            Ok(default_config)
        }
    }
}

async fn handle_template_command(
    wrapper: &mut EnhancedLLMWrapper,
    action: TemplateAction,
) -> anyhow::Result<()> {
    match action {
        TemplateAction::List => {
            let templates = wrapper.list_templates();
            if templates.is_empty() {
                println!("No templates found");
            } else {
                println!("📝 Available Templates:");
                println!("═══════════════════════");
                for template in templates {
                    println!("  📄 {}", template.name);
                    if let Some(desc) = &template.description {
                        println!("     {}", desc);
                    }
                    println!("     Variables: {}", 
                        template.variables.iter()
                            .map(|v| format!("{}{}", v.name, if v.required { "*" } else { "" }))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    println!();
                }
            }
        }
        TemplateAction::Create { name, file, description } => {
            let content = tokio::fs::read_to_string(&file).await?;
            let template = Template {
                name: name.clone(),
                content,
                description,
                variables: Vec::new(), // TODO: Parse variables from template
                created_at: std::time::SystemTime::now(),
                parent_template: None,
                tags: Vec::new(),
                usage_examples: Vec::new(),
            };
            
            wrapper.save_template(template).await?;
            println!("✅ Template '{}' created successfully", name);
        }
        TemplateAction::Show { name } => {
            let templates = wrapper.list_templates();
            if let Some(template) = templates.iter().find(|t| t.name == name) {
                println!("📄 Template: {}", template.name);
                println!("═══════════════════════");
                if let Some(desc) = &template.description {
                    println!("Description: {}", desc);
                }
                println!("Created: {:?}", template.created_at);
                println!("Variables:");
                for var in &template.variables {
                    println!("  - {} ({}{})", 
                        var.name, 
                        format!("{:?}", var.var_type).to_lowercase(),
                        if var.required { ", required" } else { "" }
                    );
                }
                println!("\nContent:");
                println!("{}", template.content);
            } else {
                println!("❌ Template '{}' not found", name);
            }
        }
        TemplateAction::Delete { name } => {
            // TODO: Implement template deletion
            println!("❌ Template deletion not yet implemented");
        }
    }
    Ok(())
}

async fn handle_cache_command(
    wrapper: &mut EnhancedLLMWrapper,
    action: CacheAction,
) -> anyhow::Result<()> {
    match action {
        CacheAction::Stats => {
            let stats = wrapper.get_cache_stats();
            println!("💾 Cache Statistics:");
            println!("═══════════════════");
            println!("Hit Ratio: {:.1}%", stats.hit_ratio() * 100.0);
            println!("Total Entries: {}", stats.total_entries);
            println!("Memory Usage: {} bytes", stats.memory_usage_bytes);
            println!("Cache Hits: {}", stats.hits);
            println!("Cache Misses: {}", stats.misses);
            println!("Evictions: {}", stats.evictions);
            println!("Disk Reads: {}", stats.disk_reads);
            println!("Disk Writes: {}", stats.disk_writes);
        }
        CacheAction::Clear => {
            wrapper.clear_cache().await?;
            println!("✅ Cache cleared successfully");
        }
        CacheAction::ClearModel { model } => {
            wrapper.invalidate_cache_for_model(&model).await?;
            println!("✅ Cache cleared for model: {}", model);
        }
    }
    Ok(())
}