# Enhanced LLM Wrapper

A production-ready, high-performance Rust wrapper for local Large Language Models with advanced features including async streaming, intelligent caching, flexible templating, and rich terminal UI.

## 🚀 Features

### Core Capabilities
- **🔄 Async Streaming**: Real-time token streaming with cancellation support
- **💾 Intelligent Caching**: LRU cache with persistence and memory pressure handling
- **📝 Template System**: Handlebars-based templating with sandboxing and composition
- **🖥️ Rich Terminal UI**: Interactive TUI with markdown rendering and syntax highlighting
- **📊 Performance Monitoring**: Comprehensive metrics and benchmarking tools
- **🔧 Multi-Backend Support**: Ollama, LM Studio, and extensible backend system

### Advanced Features
- **⚡ Performance Optimized**: Sub-200ms first token, <10ms cache lookups, <50ms template rendering
- **🛡️ Production Ready**: Structured logging, error recovery, configuration validation
- **🎯 Load Testing**: Built-in load testing and performance regression detection
- **♿ Accessibility**: High contrast mode and responsive terminal layouts
- **🔒 Security**: Template sandboxing and input validation

## 📦 Installation

### Prerequisites
- Rust 1.70+ 
- Tokio async runtime
- Local LLM server (Ollama, LM Studio, etc.)

### Build from Source
```bash
git clone <repository-url>
cd enhanced-llm-wrapper
cargo build --release
```

### Install Binaries
```bash
# Main CLI tool
cargo install --path . --bin llm-wrapper

# Load testing utility  
cargo install --path . --bin load_test
```

## 🚀 Quick Start

### Basic Usage (Legacy Mode)
```bash
# Single message
llm-wrapper "Hello, how are you?"

# Interactive chat
llm-wrapper chat

# With specific model
llm-wrapper -m codellama "Write a Rust function"

# With images (vision models)
llm-wrapper -i image.jpg "Describe this image"
```

### Enhanced Mode
```bash
# Interactive mode with full TUI
llm-wrapper enhanced interactive

# Template-based chat
llm-wrapper enhanced chat-template greeting --vars '{"name": "Alice"}'

# View performance statistics
llm-wrapper enhanced stats

# Template management
llm-wrapper enhanced template list
llm-wrapper enhanced template create greeting greeting.hbs
```

### Configuration

Create `enhanced-config.toml`:
```toml
[cache]
max_memory_entries = 1000
ttl = "1h"
enable_persistence = true
memory_pressure_threshold = 0.8

[ui]
theme = "default"
syntax_highlighting = true
auto_scroll = true
high_contrast = false

[templates]
template_dir = "templates"
auto_reload = true

[logging]
level = "info"
format = "json"
output = "stdout"

[streaming]
max_concurrent_streams = 10
buffer_size = 8192

[backends.ollama]
backend_type = "Ollama"
base_url = "http://localhost:11434"
timeout = "30s"
retry_attempts = 3
```

## 📝 Template System

### Creating Templates

Create `templates/greeting.hbs`:
```handlebars
Hello {{name}}! 

{{#if urgent}}
🚨 This is urgent: {{message}}
{{else}}
📝 Message: {{message}}
{{/if}}

{{#each tasks}}
- [ ] {{this}}
{{/each}}
```

### Using Templates
```bash
# Via CLI
llm-wrapper enhanced chat-template greeting --vars '{
  "name": "Alice",
  "urgent": true,
  "message": "Please review the code",
  "tasks": ["Review PR", "Run tests", "Deploy"]
}'

# Via API
let variables = json!({
    "name": "Alice",
    "urgent": true,
    "message": "Please review the code",
    "tasks": ["Review PR", "Run tests", "Deploy"]
});

let response = wrapper.chat_with_template("greeting", variables, Some("llama3.2")).await?;
```

## 🖥️ Terminal UI

### Features
- **Real-time streaming** with animated progress indicators
- **Markdown rendering** with syntax highlighting for code blocks
- **Keyboard shortcuts**: F1-F4 (models), F5 (auto-scroll), F6 (high contrast)
- **Responsive layout** that adapts to terminal size
- **Message history** with search and navigation
- **Cache indicators** showing cached vs live responses

### Keyboard Shortcuts
- `Ctrl+Q` / `Ctrl+C`: Quit
- `Ctrl+L`: Clear history
- `F1-F4`: Quick model switching
- `F5`: Toggle auto-scroll
- `F6`: Toggle high contrast mode
- `↑↓`: Scroll through history
- `PgUp/PgDn`: Fast scroll
- `Home/End`: Jump to start/end

## 📊 Performance Monitoring

### Built-in Metrics
```bash
# View current performance metrics
llm-wrapper enhanced stats

# Export detailed metrics
llm-wrapper enhanced stats --export metrics.json

# Run load tests
load_test --concurrency 10 --requests 100 --output load_test_results.json
```

### Performance Targets
- **First Token Time**: < 200ms
- **Cache Lookup**: < 10ms  
- **Template Rendering**: < 50ms
- **Cache Hit Ratio**: > 80%
- **Error Rate**: < 5%

### Benchmarking
```bash
# Run comprehensive benchmarks
cargo bench

# View benchmark reports
open target/criterion/report/index.html
```

## 🔧 API Usage

### Basic Setup
```rust
use llm_wrapper::{EnhancedLLMWrapper, EnhancedConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = EnhancedConfig::load("enhanced-config.toml")?;
    
    // Initialize wrapper
    let mut wrapper = EnhancedLLMWrapper::new(config).await?;
    
    // Simple chat
    let response = wrapper.chat("Hello!", None).await?;
    println!("Response: {}", response);
    
    Ok(())
}
```

### Streaming Chat
```rust
use tokio_stream::StreamExt;

// Create streaming response
let mut stream = wrapper.chat_with_template(
    "conversation", 
    json!({"topic": "Rust programming"}),
    Some("llama3.2")
).await?;

// Process tokens as they arrive
while let Some(token) = stream.receiver.recv().await {
    print!("{}", token.content);
    if token.is_complete {
        break;
    }
}
```

### Template Management
```rust
use llm_wrapper::Template;

// Create template
let template = Template {
    name: "code_review".to_string(),
    content: r#"
Please review this {{language}} code:

```{{language}}
{{code}}
```

Focus on:
{{#each focus_areas}}
- {{this}}
{{/each}}
    "#.to_string(),
    description: Some("Code review template".to_string()),
    variables: vec![/* ... */],
    created_at: std::time::SystemTime::now(),
    parent_template: None,
    tags: vec!["development".to_string()],
    usage_examples: vec![],
};

wrapper.save_template(template).await?;
```

### Cache Management
```rust
// Get cache statistics
let stats = wrapper.get_cache_stats();
println!("Hit ratio: {:.1}%", stats.hit_ratio() * 100.0);

// Clear cache for specific model
wrapper.invalidate_cache_for_model("llama3.2").await?;

// Clear all cache
wrapper.clear_cache().await?;
```

## 🧪 Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --test integration
```

### Load Testing
```bash
# Basic load test
load_test --concurrency 5 --requests 50

# Template performance test
load_test --test-templates --concurrency 10 --requests 100

# Cache performance test  
load_test --test-cache --concurrency 20 --requests 200

# Export results
load_test --output results.json --concurrency 10 --requests 100
```

## 🔍 Troubleshooting

### Common Issues

#### Cache Performance Issues
```bash
# Check cache statistics
llm-wrapper enhanced cache stats

# Clear cache if hit ratio is low
llm-wrapper enhanced cache clear

# Adjust cache settings in config
[cache]
max_memory_entries = 2000  # Increase cache size
ttl = "2h"                 # Increase TTL
```

#### Template Rendering Errors
```bash
# Validate template syntax
llm-wrapper enhanced template show template_name

# Check template variables
# Ensure all required variables are provided
```

#### Connection Issues
```bash
# Test backend connectivity
curl http://localhost:11434/api/tags

# Check backend configuration
[backends.ollama]
base_url = "http://localhost:11434"  # Verify URL
timeout = "60s"                      # Increase timeout
```

### Performance Optimization

#### Memory Usage
- Adjust `max_memory_entries` based on available RAM
- Enable `cache_persistence` for large datasets
- Monitor `memory_pressure_threshold`

#### Response Times
- Use appropriate models for your use case
- Enable caching for repeated queries
- Optimize template complexity

#### Concurrency
- Adjust `max_concurrent_streams` based on backend capacity
- Use connection pooling for high-throughput scenarios

## 📚 Architecture

### Component Overview
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Terminal UI   │    │   CLI Interface │    │   API Library   │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                    ┌─────────────▼───────────────┐
                    │   EnhancedLLMWrapper        │
                    │   (Orchestrator)            │
                    └─────────────┬───────────────┘
                                  │
        ┌─────────────────────────┼─────────────────────────┐
        │                         │                         │
┌───────▼────────┐    ┌──────────▼──────────┐    ┌─────────▼────────┐
│ Cache Manager  │    │  Template Engine    │    │ Streaming Manager│
└────────────────┘    └─────────────────────┘    └──────────────────┘
        │                         │                         │
        │              ┌──────────▼──────────┐              │
        │              │  Performance        │              │
        │              │  Monitor            │              │
        │              └─────────────────────┘              │
        │                                                   │
        └─────────────────────┬─────────────────────────────┘
                              │
                    ┌─────────▼───────────────┐
                    │   Backend Abstraction   │
                    └─────────┬───────────────┘
                              │
        ┌─────────────────────┼─────────────────────────┐
        │                     │                         │
┌───────▼────────┐  ┌─────────▼────────┐  ┌─────────────▼──────┐
│ Ollama Backend │  │ LMStudio Backend │  │  Custom Backends   │
└────────────────┘  └──────────────────┘  └────────────────────┘
```

### Data Flow
1. **Request Processing**: CLI/UI → EnhancedLLMWrapper
2. **Template Rendering**: Template Engine processes Handlebars templates
3. **Cache Lookup**: Cache Manager checks for existing responses
4. **Backend Communication**: Streaming Manager handles LLM communication
5. **Response Processing**: Tokens streamed back through the system
6. **Cache Storage**: Responses cached for future use
7. **Performance Monitoring**: All operations tracked and analyzed

## 🤝 Contributing

### Development Setup
```bash
git clone <repository-url>
cd enhanced-llm-wrapper
cargo build
cargo test
```

### Code Style
- Follow Rust standard formatting (`cargo fmt`)
- Run Clippy for linting (`cargo clippy`)
- Maintain test coverage above 80%
- Add documentation for public APIs

### Performance Requirements
- All new features must meet performance targets
- Include benchmarks for performance-critical code
- Run load tests before submitting PRs

## 📄 License

[License information]

## 🙏 Acknowledgments

- [Ollama](https://ollama.ai/) for the excellent local LLM platform
- [Ratatui](https://github.com/ratatui-org/ratatui) for the terminal UI framework
- [Tokio](https://tokio.rs/) for the async runtime
- [Handlebars](https://handlebarsjs.com/) for the templating system