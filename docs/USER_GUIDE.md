# User Guide

## Getting Started

### Installation

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Clone and build the Enhanced LLM Wrapper**:
   ```bash
   git clone <repository-url>
   cd enhanced-llm-wrapper
   cargo build --release
   ```

3. **Install the binaries**:
   ```bash
   cargo install --path . --bin llm-wrapper
   cargo install --path . --bin load_test
   ```

### Prerequisites

- **Local LLM Server**: You need a running LLM server like Ollama or LM Studio
- **Ollama Setup** (recommended):
  ```bash
  # Install Ollama
  curl -fsSL https://ollama.ai/install.sh | sh
  
  # Pull a model
  ollama pull llama3.2
  
  # Verify it's running
  curl http://localhost:11434/api/tags
  ```

### First Run

1. **Test basic functionality**:
   ```bash
   llm-wrapper "Hello, how are you today?"
   ```

2. **Try interactive mode**:
   ```bash
   llm-wrapper chat
   ```

3. **Use enhanced features**:
   ```bash
   llm-wrapper enhanced interactive
   ```

## Configuration

### Basic Configuration

The wrapper will create a default configuration file on first run. You can customize it by creating `enhanced-config.toml`:

```toml
# Cache settings
[cache]
max_memory_entries = 1000
ttl = "1h"
enable_persistence = true
cache_dir = ".cache"
memory_pressure_threshold = 0.8

# UI settings
[ui]
theme = "default"
syntax_highlighting = true
auto_scroll = true
max_history = 1000
high_contrast = false

# Template settings
[templates]
template_dir = "templates"
auto_reload = true
custom_helpers = ["upper", "lower", "format_date"]

# Logging settings
[logging]
level = "info"
format = "text"
output = "stdout"

# Streaming settings
[streaming]
max_concurrent_streams = 10
buffer_size = 8192

# Backend configurations
[backends.ollama]
backend_type = "Ollama"
base_url = "http://localhost:11434"
timeout = "30s"
retry_attempts = 3

[backends.ollama.rate_limit]
max_concurrent = 5
requests_per_minute = 60
```

### Advanced Configuration

#### Multiple Backends
```toml
[backends.ollama]
backend_type = "Ollama"
base_url = "http://localhost:11434"
timeout = "30s"

[backends.lmstudio]
backend_type = "LMStudio"
base_url = "http://localhost:1234"
timeout = "60s"

[backends.remote]
backend_type = "Custom"
base_url = "https://api.example.com"
timeout = "120s"
```

#### Performance Tuning
```toml
[cache]
max_memory_entries = 5000      # Increase for more caching
ttl = "2h"                     # Longer TTL for stable content
max_memory_bytes = 209715200   # 200MB cache limit
memory_pressure_threshold = 0.9 # Use more memory before eviction

[streaming]
max_concurrent_streams = 20    # Higher concurrency
buffer_size = 16384           # Larger buffers for better throughput
```

## Using the CLI

### Basic Commands

#### Simple Chat
```bash
# Single message
llm-wrapper "Explain quantum computing"

# With specific model
llm-wrapper -m codellama "Write a Python function to sort a list"

# With system prompt
llm-wrapper -s "You are a helpful coding assistant" "How do I handle errors in Rust?"
```

#### Interactive Mode
```bash
# Basic interactive mode
llm-wrapper chat

# Enhanced interactive mode with TUI
llm-wrapper enhanced interactive
```

#### Model Management
```bash
# List available models
llm-wrapper list

# Get model information
llm-wrapper info llama3.2

# Pull a new model (if supported by backend)
llm-wrapper pull mistral
```

### Enhanced Features

#### Template Management
```bash
# List templates
llm-wrapper enhanced template list

# Create a template
llm-wrapper enhanced template create greeting greeting.hbs

# Show template details
llm-wrapper enhanced template show greeting

# Use a template
llm-wrapper enhanced chat-template greeting --vars '{"name": "Alice", "time": "morning"}'
```

#### Cache Management
```bash
# Show cache statistics
llm-wrapper enhanced cache stats

# Clear all cache
llm-wrapper enhanced cache clear

# Clear cache for specific model
llm-wrapper enhanced cache clear-model llama3.2
```

#### Performance Monitoring
```bash
# Show performance statistics
llm-wrapper enhanced stats

# Export detailed metrics
llm-wrapper enhanced stats --export metrics.json
```

## Templates

### Creating Templates

Templates use Handlebars syntax and are stored in the `templates/` directory.

#### Basic Template (`templates/greeting.hbs`)
```handlebars
Hello {{name}}! 

{{#if urgent}}
🚨 This is urgent: {{message}}
{{else}}
📝 Message: {{message}}
{{/if}}

Have a great {{time_of_day}}!
```

#### Advanced Template (`templates/code_review.hbs`)
```handlebars
Please review this {{language}} code:

```{{language}}
{{code}}
```

{{#if focus_areas}}
Focus on:
{{#each focus_areas}}
- {{this}}
{{/each}}
{{/if}}

{{#if complexity}}
Complexity level: {{complexity}}
{{/if}}

{{#if deadline}}
⏰ Deadline: {{deadline}}
{{/if}}

Please provide:
1. Overall assessment
2. Specific issues found
3. Improvement suggestions
4. Security considerations (if applicable)
```

#### Template with Helpers (`templates/report.hbs`)
```handlebars
# {{upper title}}

Generated on: {{format_date timestamp "YYYY-MM-DD HH:mm"}}

## Summary
{{summary}}

## Data
{{#each items}}
- **{{upper name}}**: {{value}} ({{lower unit}})
{{/each}}

## Status
{{#if (gt score 80)}}
✅ Excellent performance
{{else if (gt score 60)}}
⚠️ Good performance with room for improvement
{{else}}
❌ Performance needs attention
{{/if}}
```

### Using Templates

#### Via CLI
```bash
# Simple template
llm-wrapper enhanced chat-template greeting --vars '{
  "name": "Alice",
  "message": "Welcome to the team!",
  "urgent": false,
  "time_of_day": "morning"
}'

# Complex template
llm-wrapper enhanced chat-template code_review --vars '{
  "language": "rust",
  "code": "fn fibonacci(n: u32) -> u32 {\n    if n <= 1 { n } else { fibonacci(n-1) + fibonacci(n-2) }\n}",
  "focus_areas": ["performance", "algorithm efficiency", "memory usage"],
  "complexity": "medium",
  "deadline": "2024-01-15"
}'
```

#### Via API
```rust
use serde_json::json;

let variables = json!({
    "name": "Alice",
    "message": "Welcome to the team!",
    "urgent": false,
    "time_of_day": "morning"
});

let response = wrapper.chat_with_template("greeting", variables, Some("llama3.2")).await?;
```

### Template Best Practices

1. **Keep templates focused**: Each template should serve a specific purpose
2. **Use descriptive variable names**: Make it clear what each variable represents
3. **Provide defaults**: Use Handlebars helpers to provide sensible defaults
4. **Document variables**: Include comments explaining expected variables
5. **Test templates**: Verify templates work with various input combinations

## Terminal UI

### Features

The enhanced terminal UI provides a rich interactive experience:

- **Real-time streaming**: See responses as they're generated
- **Markdown rendering**: Properly formatted text with syntax highlighting
- **Message history**: Navigate through previous conversations
- **Cache indicators**: See which responses came from cache
- **Responsive layout**: Adapts to different terminal sizes
- **Accessibility**: High contrast mode and keyboard navigation

### Keyboard Shortcuts

#### Navigation
- `↑` / `↓`: Scroll through message history
- `Page Up` / `Page Down`: Fast scroll
- `Home`: Jump to beginning of history
- `End`: Jump to end of history

#### Actions
- `Enter`: Send message
- `Ctrl+L`: Clear message history
- `Ctrl+Q` / `Ctrl+C`: Quit application
- `Esc`: Quit application

#### Model Management
- `F1`: Switch to llama3.2
- `F2`: Switch to codellama
- `F3`: Switch to mistral
- `F4`: Switch to phi3

#### UI Options
- `F5`: Toggle auto-scroll
- `F6`: Toggle high contrast mode

### Usage Tips

1. **Long messages**: The UI automatically wraps long messages and provides scrolling
2. **Code blocks**: Code in responses is syntax highlighted automatically
3. **Status indicators**: 
   - 🔴 Live: Currently receiving streaming response
   - ⚫ Ready: Ready for new input
   - 📋: Response came from cache
   - 📝: Response used a template
4. **Performance info**: Status bar shows cache hit rate and current model

## Performance Optimization

### Cache Optimization

#### Monitoring Cache Performance
```bash
# Check cache statistics
llm-wrapper enhanced cache stats
```

Look for:
- **Hit ratio**: Should be > 80% for optimal performance
- **Memory usage**: Should stay within configured limits
- **Eviction rate**: High evictions may indicate need for larger cache

#### Tuning Cache Settings
```toml
[cache]
# Increase cache size for better hit rates
max_memory_entries = 2000

# Longer TTL for stable content
ttl = "2h"

# Increase memory limit
max_memory_bytes = 419430400  # 400MB

# Allow higher memory usage before eviction
memory_pressure_threshold = 0.9
```

### Template Performance

#### Optimizing Templates
1. **Minimize complexity**: Avoid deeply nested loops and conditionals
2. **Cache compiled templates**: Templates are automatically cached after first use
3. **Use simple helpers**: Complex custom helpers can slow rendering

#### Monitoring Template Performance
```bash
# Check template render times
llm-wrapper enhanced stats
```

Target: < 50ms average render time

### Backend Performance

#### Connection Optimization
```toml
[backends.ollama]
timeout = "60s"        # Increase for slow models
retry_attempts = 5     # More retries for reliability

[backends.ollama.rate_limit]
max_concurrent = 10    # Increase for better throughput
requests_per_minute = 120  # Higher rate limit
```

#### Model Selection
- **Fast models**: Use smaller models (7B parameters) for quick responses
- **Quality models**: Use larger models (13B+ parameters) for complex tasks
- **Specialized models**: Use code-specific models for programming tasks

### System Performance

#### Memory Management
```bash
# Monitor memory usage
llm-wrapper enhanced stats

# Clear cache if memory usage is high
llm-wrapper enhanced cache clear
```

#### Concurrent Usage
```toml
[streaming]
max_concurrent_streams = 20  # Increase for high concurrency
buffer_size = 32768         # Larger buffers for better throughput
```

## Troubleshooting

### Common Issues

#### Connection Problems
```bash
# Test backend connectivity
curl http://localhost:11434/api/tags

# Check if Ollama is running
ps aux | grep ollama

# Restart Ollama if needed
ollama serve
```

#### Cache Issues
```bash
# Clear corrupted cache
llm-wrapper enhanced cache clear

# Check cache directory permissions
ls -la .cache/

# Disable persistence if disk issues
# In config: enable_persistence = false
```

#### Template Errors
```bash
# Validate template syntax
llm-wrapper enhanced template show template_name

# Check template directory
ls -la templates/

# Verify template variables match usage
```

#### Performance Issues
```bash
# Check performance metrics
llm-wrapper enhanced stats

# Run load test to identify bottlenecks
load_test --concurrency 5 --requests 50 --output results.json

# Monitor system resources
top -p $(pgrep llm-wrapper)
```

### Error Messages

#### "Backend not found"
- Check backend configuration in `enhanced-config.toml`
- Verify backend server is running
- Test connectivity with curl

#### "Template not found"
- Check template exists in templates directory
- Verify template name spelling
- List available templates: `llm-wrapper enhanced template list`

#### "Cache error"
- Check disk space in cache directory
- Verify write permissions
- Clear cache: `llm-wrapper enhanced cache clear`

#### "Stream timeout"
- Increase timeout in backend configuration
- Check network connectivity
- Try with smaller/faster model

### Performance Troubleshooting

#### Slow Response Times
1. **Check model size**: Smaller models respond faster
2. **Monitor cache hit rate**: Low hit rate means more backend calls
3. **Check system resources**: High CPU/memory usage affects performance
4. **Network latency**: Test with local vs remote backends

#### High Memory Usage
1. **Reduce cache size**: Lower `max_memory_entries`
2. **Enable memory pressure handling**: Lower `memory_pressure_threshold`
3. **Clear cache regularly**: Set shorter TTL or manual clearing
4. **Disable persistence**: If disk I/O is causing issues

#### Template Rendering Slow
1. **Simplify templates**: Reduce complexity of conditionals and loops
2. **Check helper performance**: Custom helpers may be slow
3. **Monitor template cache**: Templates should be cached after first use

### Getting Help

#### Debug Information
```bash
# Enable debug logging
export RUST_LOG=debug
llm-wrapper enhanced interactive

# Export performance metrics
llm-wrapper enhanced stats --export debug_metrics.json

# Run with verbose output
llm-wrapper -v enhanced stats
```

#### Log Analysis
```bash
# Check application logs
tail -f llm-wrapper.log

# Filter for errors
grep ERROR llm-wrapper.log

# Check performance logs
grep "Performance metric" llm-wrapper.log
```

#### Community Support
- Check documentation for similar issues
- Search existing GitHub issues
- Create detailed bug report with:
  - Configuration file
  - Error messages
  - Performance metrics
  - System information

## Advanced Usage

### Custom Backends

You can implement custom backends by implementing the `Backend` trait:

```rust
use async_trait::async_trait;
use llm_wrapper::{Backend, BackendError, ModelInfo, ModelCapabilities, ChatRequest, StreamResponse};

pub struct CustomBackend {
    base_url: String,
    client: reqwest::Client,
}

#[async_trait]
impl Backend for CustomBackend {
    async fn chat(&self, request: ChatRequest) -> Result<String, BackendError> {
        // Implement chat logic
        todo!()
    }
    
    async fn chat_stream(&self, request: ChatRequest) -> Result<StreamResponse, BackendError> {
        // Implement streaming logic
        todo!()
    }
    
    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        // Implement model listing
        todo!()
    }
    
    fn capabilities(&self) -> &ModelCapabilities {
        // Return backend capabilities
        todo!()
    }
    
    fn backend_type(&self) -> crate::backends::BackendType {
        crate::backends::BackendType::Custom
    }
}
```

### Custom Template Helpers

Register custom Handlebars helpers:

```rust
use handlebars::{Helper, RenderContext, RenderError, Output};

// Custom helper function
fn format_currency(
    h: &Helper,
    _: &handlebars::Handlebars,
    _: &handlebars::Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let amount = h.param(0)
        .and_then(|v| v.value().as_f64())
        .ok_or_else(|| RenderError::new("Invalid amount"))?;
    
    let formatted = format!("${:.2}", amount);
    out.write(&formatted)?;
    Ok(())
}

// Register helper
template_engine.register_helper("currency", Box::new(format_currency));
```

### Integration with External Systems

#### Webhook Integration
```rust
use axum::{extract::Json, http::StatusCode, response::Json as ResponseJson, routing::post, Router};

async fn webhook_handler(
    Json(payload): Json<WebhookPayload>,
) -> Result<ResponseJson<WebhookResponse>, StatusCode> {
    let mut wrapper = get_wrapper().await;
    
    let response = wrapper.chat_with_template(
        "webhook_response",
        serde_json::to_value(payload)?,
        None
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(ResponseJson(WebhookResponse { 
        message: response.content 
    }))
}
```

#### Database Integration
```rust
use sqlx::{PgPool, Row};

async fn process_database_queries(pool: &PgPool, wrapper: &mut EnhancedLLMWrapper) -> Result<(), Box<dyn std::error::Error>> {
    let rows = sqlx::query("SELECT id, query FROM pending_queries")
        .fetch_all(pool)
        .await?;
    
    for row in rows {
        let id: i32 = row.get("id");
        let query: String = row.get("query");
        
        let response = wrapper.chat(&query, None).await?;
        
        sqlx::query("UPDATE pending_queries SET response = $1, processed_at = NOW() WHERE id = $2")
            .bind(&response)
            .bind(id)
            .execute(pool)
            .await?;
    }
    
    Ok(())
}
```

This user guide covers the essential aspects of using the Enhanced LLM Wrapper effectively. For more detailed technical information, refer to the API documentation.