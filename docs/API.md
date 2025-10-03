# API Documentation

## Overview

The Enhanced LLM Wrapper provides a comprehensive Rust API for interacting with local Large Language Models. This document covers all public APIs, data structures, and usage patterns.

## Core Types

### EnhancedLLMWrapper

The main orchestrator that combines all system components.

```rust
pub struct EnhancedLLMWrapper {
    // Internal fields are private
}

impl EnhancedLLMWrapper {
    /// Create a new wrapper instance with the given configuration
    pub async fn new(config: EnhancedConfig) -> Result<Self, WrapperError>;
    
    /// Chat with template rendering and caching
    pub async fn chat_with_template(
        &mut self,
        template_name: &str,
        variables: serde_json::Value,
        model: Option<&str>,
    ) -> Result<StreamResponse, WrapperError>;
    
    /// Simple chat without templates
    pub async fn chat(
        &mut self,
        message: &str,
        model: Option<&str>,
    ) -> Result<String, WrapperError>;
    
    /// Launch interactive terminal UI mode
    pub async fn interactive_mode(&mut self) -> Result<(), WrapperError>;
    
    /// Switch to a different backend
    pub fn switch_backend(&mut self, backend_name: &str) -> Result<(), WrapperError>;
    
    /// List available backends
    pub fn list_backends(&self) -> Vec<&str>;
    
    /// List available models from current backend
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, WrapperError>;
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> &CacheStats;
    
    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> PerformanceMetrics;
    
    /// Get performance report with target analysis
    pub fn get_performance_report(&self) -> PerformanceReport;
    
    /// Export performance metrics to file
    pub async fn export_performance_metrics(&self, path: &str) -> Result<(), WrapperError>;
    
    /// List available templates
    pub fn list_templates(&self) -> Vec<&Template>;
    
    /// Save a new template
    pub async fn save_template(&mut self, template: Template) -> Result<(), WrapperError>;
    
    /// Clear all cache entries
    pub async fn clear_cache(&mut self) -> Result<(), WrapperError>;
    
    /// Clear cache entries for a specific model
    pub async fn invalidate_cache_for_model(&mut self, model: &str) -> Result<(), WrapperError>;
}
```

### Configuration Types

#### EnhancedConfig

Main configuration structure for the entire system.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedConfig {
    pub backends: HashMap<String, BackendConfig>,
    pub cache: CacheConfig,
    pub ui: UIConfig,
    pub templates: TemplateConfig,
    pub logging: LoggingConfig,
    pub streaming: StreamingConfig,
}

impl EnhancedConfig {
    /// Load configuration from TOML file with validation
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError>;
    
    /// Save configuration to TOML file
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ConfigError>;
    
    /// Validate configuration settings
    fn validate(&self) -> Result<(), ConfigError>;
}
```

#### BackendConfig

Configuration for individual LLM backends.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub backend_type: BackendType,
    pub base_url: String,
    pub timeout: Duration,
    pub retry_attempts: u32,
    pub rate_limit: Option<RateLimit>,
    pub default_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendType {
    Ollama,
    LMStudio,
    OpenAI,
    Custom,
    Mock,
}
```

#### CacheConfig

Configuration for the intelligent caching system.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_memory_entries: usize,
    pub ttl: Duration,
    pub enable_persistence: bool,
    pub cache_streaming: bool,
    pub cache_dir: Option<PathBuf>,
    pub max_memory_bytes: Option<usize>,
    pub memory_pressure_threshold: f64,
}
```

### Streaming Types

#### StreamResponse

Represents an active streaming response from an LLM.

```rust
pub struct StreamResponse {
    pub id: StreamId,
    pub receiver: mpsc::UnboundedReceiver<StreamToken>,
    pub cancellation_token: CancellationToken,
}
```

#### StreamToken

Individual token in a streaming response.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamToken {
    pub content: String,
    pub is_complete: bool,
    pub metadata: Option<TokenMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub token_count: Option<u32>,
}
```

### Template Types

#### Template

Represents a Handlebars template with metadata.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub content: String,
    pub description: Option<String>,
    pub variables: Vec<TemplateVariable>,
    pub created_at: SystemTime,
    pub parent_template: Option<String>,
    pub tags: Vec<String>,
    pub usage_examples: Vec<String>,
}
```

#### TemplateVariable

Describes a variable used in a template.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: VariableType,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariableType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}
```

#### TemplateEngine

Manages template compilation and rendering.

```rust
impl TemplateEngine {
    /// Create new template engine with configuration
    pub fn new(config: TemplateConfig) -> Self;
    
    /// Render a template with the given context
    pub fn render(&self, template_name: &str, context: &serde_json::Value) -> Result<String, TemplateError>;
    
    /// Register a new template
    pub fn register_template(&mut self, template: Template) -> Result<(), TemplateError>;
    
    /// List all available templates
    pub fn list_templates(&self) -> Vec<&Template>;
    
    /// Validate template syntax
    pub fn validate_template(&self, content: &str) -> Result<(), TemplateError>;
}
```

### Cache Types

#### CacheManager

Manages intelligent caching with LRU eviction and persistence.

```rust
impl CacheManager {
    /// Create new cache manager
    pub fn new(config: CacheConfig) -> Self;
    
    /// Create cache manager with persistence support
    pub async fn new_with_persistence(config: CacheConfig) -> Result<Self, CacheError>;
    
    /// Get cached response
    pub async fn get(&mut self, key: &CacheKey) -> Option<String>;
    
    /// Store response in cache
    pub async fn put(&mut self, key: CacheKey, value: String, metadata: ResponseMetadata) -> Result<(), CacheError>;
    
    /// Invalidate all entries for a model
    pub fn invalidate_model(&mut self, model: &str);
    
    /// Get cache statistics
    pub fn get_stats(&self) -> &CacheStats;
    
    /// Clear all cache entries
    pub fn clear(&mut self);
    
    /// Persist cache to disk
    pub async fn persist_to_disk(&mut self) -> Result<(), CacheError>;
}
```

#### CacheStats

Statistics about cache performance.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_entries: usize,
    pub memory_usage_bytes: usize,
    pub evictions: u64,
    pub disk_writes: u64,
    pub disk_reads: u64,
}

impl CacheStats {
    /// Calculate hit ratio (0.0 to 1.0)
    pub fn hit_ratio(&self) -> f64;
    
    /// Calculate eviction ratio
    pub fn eviction_ratio(&self) -> f64;
}
```

### Performance Monitoring

#### PerformanceMonitor

Tracks system performance metrics and targets.

```rust
impl PerformanceMonitor {
    /// Create new performance monitor
    pub fn new() -> Self;
    
    /// Record operation timing
    pub fn record_operation_time(&self, operation: &str, duration: Duration);
    
    /// Increment counter
    pub fn increment_counter(&self, counter: &str);
    
    /// Record cache operation
    pub fn record_cache_operation(&self, operation_type: &str, duration: Duration, success: bool);
    
    /// Record template render
    pub fn record_template_render(&self, duration: Duration, success: bool);
    
    /// Get current metrics
    pub fn get_metrics(&self) -> PerformanceMetrics;
    
    /// Check performance against targets
    pub fn check_performance_targets(&self) -> PerformanceReport;
    
    /// Export metrics to file
    pub async fn export_metrics_to_file(&self, path: &str) -> Result<(), std::io::Error>;
}
```

#### PerformanceMetrics

Comprehensive performance metrics.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cache_metrics: CachePerformanceMetrics,
    pub template_metrics: TemplatePerformanceMetrics,
    pub streaming_metrics: StreamingPerformanceMetrics,
    pub system_metrics: SystemPerformanceMetrics,
}
```

### Error Types

#### WrapperError

Main error type for the wrapper system.

```rust
#[derive(Debug, Error)]
pub enum WrapperError {
    #[error("Backend error: {0}")]
    Backend(#[from] BackendError),
    
    #[error("Backend initialization error: {0}")]
    BackendInit(#[from] BackendInitError),
    
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),
    
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),
    
    #[error("UI error: {0}")]
    UI(#[from] UIError),
    
    #[error("Stream error: {0}")]
    Stream(#[from] StreamError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

## Usage Examples

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
    let response = wrapper.chat("Hello, world!", None).await?;
    println!("Response: {}", response);
    
    Ok(())
}
```

### Streaming Chat

```rust
use tokio_stream::StreamExt;
use serde_json::json;

// Create streaming response
let stream_response = wrapper.chat_with_template(
    "conversation",
    json!({"topic": "Rust programming", "level": "beginner"}),
    Some("llama3.2")
).await?;

// Process tokens as they arrive
let mut receiver = stream_response.receiver;
while let Some(token) = receiver.recv().await {
    print!("{}", token.content);
    std::io::Write::flush(&mut std::io::stdout())?;
    
    if token.is_complete {
        println!(); // New line after completion
        break;
    }
}
```

### Template Management

```rust
use llm_wrapper::{Template, TemplateVariable, VariableType};
use std::time::SystemTime;

// Create a new template
let template = Template {
    name: "code_review".to_string(),
    content: r#"
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

{{#if severity}}
Severity: {{severity}}
{{/if}}
    "#.to_string(),
    description: Some("Template for code review requests".to_string()),
    variables: vec![
        TemplateVariable {
            name: "language".to_string(),
            var_type: VariableType::String,
            required: true,
            default_value: None,
            description: Some("Programming language".to_string()),
        },
        TemplateVariable {
            name: "code".to_string(),
            var_type: VariableType::String,
            required: true,
            default_value: None,
            description: Some("Code to review".to_string()),
        },
        TemplateVariable {
            name: "focus_areas".to_string(),
            var_type: VariableType::Array,
            required: false,
            default_value: None,
            description: Some("Areas to focus on".to_string()),
        },
    ],
    created_at: SystemTime::now(),
    parent_template: None,
    tags: vec!["development".to_string(), "review".to_string()],
    usage_examples: vec![
        r#"{"language": "rust", "code": "fn main() {}", "focus_areas": ["performance", "safety"]}"#.to_string()
    ],
};

// Save template
wrapper.save_template(template).await?;

// Use template
let variables = json!({
    "language": "rust",
    "code": "fn fibonacci(n: u32) -> u32 {\n    if n <= 1 { n } else { fibonacci(n-1) + fibonacci(n-2) }\n}",
    "focus_areas": ["performance", "algorithm efficiency"],
    "severity": "medium"
});

let response = wrapper.chat_with_template("code_review", variables, Some("codellama")).await?;
```

### Cache Management

```rust
// Get cache statistics
let stats = wrapper.get_cache_stats();
println!("Cache hit ratio: {:.1}%", stats.hit_ratio() * 100.0);
println!("Total entries: {}", stats.total_entries);
println!("Memory usage: {} bytes", stats.memory_usage_bytes);

// Clear cache for specific model
wrapper.invalidate_cache_for_model("llama3.2").await?;

// Clear all cache
wrapper.clear_cache().await?;
```

### Performance Monitoring

```rust
// Get performance metrics
let metrics = wrapper.get_performance_metrics();
println!("Average cache lookup: {:.2}ms", metrics.cache_metrics.average_lookup_time_ms);
println!("Template render time: {:.2}ms", metrics.template_metrics.average_render_time_ms);

// Check performance targets
let report = wrapper.get_performance_report();
match report.overall_status {
    PerformanceStatus::Good => println!("✅ All targets met"),
    PerformanceStatus::Warning => {
        println!("⚠️ Performance warnings:");
        for issue in &report.issues {
            println!("  - {}", issue);
        }
    },
    PerformanceStatus::Critical => {
        println!("🚨 Critical performance issues:");
        for issue in &report.issues {
            println!("  - {}", issue);
        }
    }
}

// Export metrics
wrapper.export_performance_metrics("metrics.json").await?;
```

### Backend Management

```rust
// List available backends
let backends = wrapper.list_backends();
println!("Available backends: {:?}", backends);

// Switch backend
wrapper.switch_backend("lmstudio")?;

// List models from current backend
let models = wrapper.list_models().await?;
for model in models {
    println!("Model: {} ({})", model.name, model.size.unwrap_or(0));
}
```

### Error Handling

```rust
use llm_wrapper::{WrapperError, BackendError, TemplateError};

match wrapper.chat_with_template("nonexistent", json!({}), None).await {
    Ok(response) => {
        // Handle successful response
    },
    Err(WrapperError::Template(TemplateError::NotFound(name))) => {
        eprintln!("Template '{}' not found", name);
    },
    Err(WrapperError::Backend(BackendError::Connection(msg))) => {
        eprintln!("Backend connection failed: {}", msg);
    },
    Err(WrapperError::Cache(cache_err)) => {
        eprintln!("Cache error: {}", cache_err);
    },
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
    }
}
```

## Performance Considerations

### Memory Management
- Configure `max_memory_entries` based on available RAM
- Use `memory_pressure_threshold` to prevent OOM conditions
- Enable persistence for large cache datasets

### Concurrency
- Adjust `max_concurrent_streams` based on backend capacity
- Use appropriate `buffer_size` for streaming operations
- Consider rate limiting for high-throughput scenarios

### Caching Strategy
- Set appropriate TTL based on content freshness requirements
- Monitor hit ratios and adjust cache size accordingly
- Use model-specific invalidation for targeted cache clearing

### Template Optimization
- Keep templates simple to minimize render time
- Use template composition for reusable components
- Enable sandboxing for security in multi-tenant environments

## Thread Safety

The Enhanced LLM Wrapper is designed for single-threaded async usage. For multi-threaded scenarios:

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let wrapper = Arc::new(Mutex::new(wrapper));

// Clone for use in different tasks
let wrapper_clone = Arc::clone(&wrapper);
tokio::spawn(async move {
    let mut w = wrapper_clone.lock().await;
    let response = w.chat("Hello", None).await.unwrap();
    println!("Response: {}", response);
});
```

## Integration Patterns

### Web Server Integration

```rust
use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

type SharedWrapper = Arc<Mutex<EnhancedLLMWrapper>>;

async fn chat_endpoint(
    State(wrapper): State<SharedWrapper>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    let mut w = wrapper.lock().await;
    
    match w.chat(&request.message, request.model.as_deref()).await {
        Ok(response) => Ok(Json(ChatResponse { response })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[tokio::main]
async fn main() {
    let config = EnhancedConfig::default();
    let wrapper = Arc::new(Mutex::new(EnhancedLLMWrapper::new(config).await.unwrap()));
    
    let app = Router::new()
        .route("/chat", post(chat_endpoint))
        .with_state(wrapper);
    
    // Run server...
}
```

### CLI Tool Integration

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Chat { message: String },
    Template { name: String, vars: String },
    Stats,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = EnhancedConfig::load("config.toml")?;
    let mut wrapper = EnhancedLLMWrapper::new(config).await?;
    
    match cli.command {
        Commands::Chat { message } => {
            let response = wrapper.chat(&message, None).await?;
            println!("{}", response);
        },
        Commands::Template { name, vars } => {
            let variables: serde_json::Value = serde_json::from_str(&vars)?;
            let response = wrapper.chat_with_template(&name, variables, None).await?;
            // Handle streaming response...
        },
        Commands::Stats => {
            let metrics = wrapper.get_performance_metrics();
            println!("{:#?}", metrics);
        }
    }
    
    Ok(())
}
```