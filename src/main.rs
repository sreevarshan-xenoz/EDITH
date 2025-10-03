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
            interactive_mode(wrapper).await?;
        }
        Some(Commands::Info { model }) => {
            let model_name = model.as_deref().unwrap_or(&cli.model);
            wrapper.switch_model(model_name).await?;
            let caps = wrapper.capabilities();
            println!("Model: {}", caps.model_name);
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
                interactive_mode(wrapper).await?;
            }
        }
    }
    
    Ok(())
}

async fn interactive_mode(mut wrapper: LLMWrapper) -> anyhow::Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use ratatui::{
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
        Terminal,
    };
    use std::io;
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let mut messages: Vec<String> = Vec::new();
    let mut input = String::new();
    let mut current_images: Vec<PathBuf> = Vec::new();
    
    // Show capabilities
    let caps = wrapper.capabilities();
    messages.push(format!("🤖 Connected to: {}", caps.model_name));
    messages.push(format!("📷 Vision: {} | 🧠 Thinking: {} | 💬 Streaming: {}", 
        if caps.supports_vision { "✅" } else { "❌" },
        if caps.supports_thinking { "✅" } else { "❌" },
        if caps.supports_streaming { "✅" } else { "❌" }
    ));
    messages.push("Commands: /image <path>, /model <name>, /clear, /quit".to_string());
    messages.push("-".repeat(50));
    
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(f.size());
            
            // Chat area
            let chat_items: Vec<ListItem> = messages
                .iter()
                .map(|m| ListItem::new(Line::from(Span::raw(m))))
                .collect();
            
            let chat = List::new(chat_items)
                .block(Block::default().borders(Borders::ALL).title("Chat"));
            f.render_widget(chat, chunks[0]);
            
            // Input area
            let input_text = if current_images.is_empty() {
                format!("💬 {}", input)
            } else {
                format!("💬📷({}) {}", current_images.len(), input)
            };
            
            let input_widget = Paragraph::new(input_text)
                .block(Block::default().borders(Borders::ALL).title("Input"))
                .wrap(Wrap { trim: true });
            f.render_widget(input_widget, chunks[1]);
        })?;
        
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Enter => {
                        if input.trim().is_empty() {
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
                                            messages.push(format!("📷 Added: {}", path.display()));
                                        } else {
                                            messages.push(format!("❌ File not found: {}", parts[1]));
                                        }
                                    }
                                }
                                "/model" => {
                                    if parts.len() > 1 {
                                        match wrapper.switch_model(parts[1]).await {
                                            Ok(_) => {
                                                let caps = wrapper.capabilities();
                                                messages.push(format!("✅ Switched to: {}", caps.model_name));
                                            }
                                            Err(e) => messages.push(format!("❌ Error: {}", e)),
                                        }
                                    }
                                }
                                "/clear" => {
                                    current_images.clear();
                                    messages.push("🗑️ Cleared images".to_string());
                                }
                                _ => messages.push(format!("❌ Unknown command: {}", parts[0])),
                            }
                        } else {
                            // Send message
                            messages.push(format!("👤 You: {}", input));
                            
                            match wrapper.chat(&input, &current_images, None).await {
                                Ok(response) => {
                                    messages.push(format!("🤖 Assistant: {}", response));
                                }
                                Err(e) => {
                                    messages.push(format!("❌ Error: {}", e));
                                }
                            }
                            
                            current_images.clear();
                        }
                        
                        input.clear();
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
    }
    
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    
    Ok(())
}