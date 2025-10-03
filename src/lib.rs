use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};

use mime_guess::from_path;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

// New modules
pub mod streaming;
pub mod cache;
pub mod template;
pub mod ui;
pub mod error;
pub mod config;
pub mod backends;
pub mod logging;

// Re-exports
pub use error::{WrapperError, BackendError, ConfigError};
pub use config::EnhancedConfig;
pub use backends::{Backend, BackendType, ModelInfo, ModelCapabilities, OllamaBackend, MockBackend};
pub use streaming::{StreamingManager, StreamResponse, StreamToken};
pub use cache::{CacheManager, CacheStats};
pub use template::{TemplateEngine, Template};
pub use ui::{TerminalUI, ChatMessage, MessageRole};



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub vision_models: Vec<String>,
    pub thinking_models: Vec<String>,
    pub model_aliases: HashMap<String, String>,
    pub default_model: String,
    pub base_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vision_models: vec![
                "llava".to_string(),
                "bakllava".to_string(),
                "moondream".to_string(),
                "vision".to_string(),
            ],
            thinking_models: vec![
                "o1".to_string(),
                "reasoning".to_string(),
                "thinking".to_string(),
            ],
            model_aliases: HashMap::new(),
            default_model: "llama3.2".to_string(),
            base_url: "http://localhost:11434".to_string(),
        }
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    message: MessageResponse,
    #[serde(default)]
    done: bool,
}

#[derive(Debug, Deserialize, Default)]
struct MessageResponse {
    #[serde(default)]
    content: String,
    #[serde(default)]
    thinking: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelInfo {
    models: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    name: String,
}

pub struct LLMWrapper {
    client: Client,
    base_url: String,
    model: String,
    capabilities: crate::backends::ModelCapabilities,
    config: Config,
}

impl LLMWrapper {
    pub async fn new(base_url: &str, model: &str, config: Config) -> Result<Self> {
        let client = Client::new();
        let base_url = base_url.trim_end_matches('/').to_string();
        
        let mut wrapper = Self {
            client,
            base_url,
            model: model.to_string(),
            capabilities: crate::backends::ModelCapabilities::default(),
            config,
        };
        
        wrapper.detect_capabilities().await?;
        Ok(wrapper)
    }
    
    async fn detect_capabilities(&mut self) -> Result<()> {
        // Check if server is reachable
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        if response.status().is_success() {
            let model_info: OllamaModelInfo = response.json().await?;
            
            if let Some(current_model) = model_info.models.iter().find(|m| m.name.contains(&self.model)) {
                // Note: The new ModelCapabilities doesn't have model_name field
                // We'll need to track this separately or modify the structure
                let model_name_lower = current_model.name.to_lowercase();
                
                // Check for vision capabilities
                self.capabilities.supports_vision = self.config.vision_models
                    .iter()
                    .any(|indicator| model_name_lower.contains(indicator));
                
                // Check for thinking capabilities
                self.capabilities.supports_thinking = self.config.thinking_models
                    .iter()
                    .any(|indicator| model_name_lower.contains(indicator));
            }
        }
        
        Ok(())
    }
    
    pub fn capabilities(&self) -> &crate::backends::ModelCapabilities {
        &self.capabilities
    }
    
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        if response.status().is_success() {
            let model_info: OllamaModelInfo = response.json().await?;
            Ok(model_info.models.into_iter().map(|m| m.name).collect())
        } else {
            Err(anyhow!("Failed to fetch models"))
        }
    }
    
    pub async fn switch_model(&mut self, model_name: &str) -> Result<()> {
        // Check if it's an alias
        let actual_model = self.config.model_aliases
            .get(model_name)
            .map(|s| s.as_str())
            .unwrap_or(model_name);
        
        self.model = actual_model.to_string();
        self.detect_capabilities().await?;
        Ok(())
    }
    
    async fn encode_image<P: AsRef<Path>>(&self, image_path: P) -> Result<String> {
        let content = fs::read(&image_path).await?;
        Ok(general_purpose::STANDARD.encode(content))
    }
    
    fn is_image_file<P: AsRef<Path>>(&self, file_path: P) -> bool {
        if let Some(mime) = from_path(&file_path).first() {
            mime.type_() == mime::IMAGE
        } else {
            false
        }
    }
    
    pub async fn chat(&self, message: &str, images: &[PathBuf], system_prompt: Option<&str>) -> Result<String> {
        let mut messages = Vec::new();
        
        // Add system message if provided
        if let Some(system) = system_prompt {
            messages.push(Message {
                role: "system".to_string(),
                content: system.to_string(),
                images: None,
            });
        }
        
        // Build user message
        let mut user_message = Message {
            role: "user".to_string(),
            content: message.to_string(),
            images: None,
        };
        
        // Handle images if model supports vision
        if !images.is_empty() && self.capabilities.supports_vision {
            let mut image_data = Vec::new();
            for img_path in images {
                if img_path.exists() && self.is_image_file(img_path) {
                    match self.encode_image(img_path).await {
                        Ok(encoded) => image_data.push(encoded),
                        Err(e) => eprintln!("⚠️  Failed to encode image {}: {}", img_path.display(), e),
                    }
                }
            }
            if !image_data.is_empty() {
                user_message.images = Some(image_data);
            }
        } else if !images.is_empty() && !self.capabilities.supports_vision {
            eprintln!("⚠️  Model doesn't support vision - ignoring images");
        }
        
        messages.push(user_message);
        
        let mut request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false, // For now, let's use non-streaming for simplicity
            options: None,
        };
        
        // Handle thinking models
        if self.capabilities.supports_thinking {
            let mut options = HashMap::new();
            options.insert("thinking".to_string(), serde_json::Value::Bool(true));
            request.options = Some(options);
        }
        
        let url = format!("{}/api/chat", self.base_url);
        let response = self.client.post(&url).json(&request).send().await?;
        
        if response.status().is_success() {
            let chat_response: ChatResponse = response.json().await?;
            
            let mut result = chat_response.message.content;
            if let Some(thinking) = chat_response.message.thinking {
                result = format!("🤔 Thinking: {}\n\n{}", thinking, result);
            }
            
            Ok(result)
        } else {
            Err(anyhow!("Chat request failed: {}", response.status()))
        }
    }
    
    pub async fn pull_model(&self, model_name: &str) -> Result<()> {
        let url = format!("{}/api/pull", self.base_url);
        let request = serde_json::json!({
            "name": model_name
        });
        
        let response = self.client.post(&url).json(&request).send().await?;
        
        if response.status().is_success() {
            println!("✅ Model {} pulled successfully", model_name);
            Ok(())
        } else {
            Err(anyhow!("Failed to pull model: {}", response.status()))
        }
    }
    
    pub async fn delete_model(&self, model_name: &str) -> Result<()> {
        let url = format!("{}/api/delete", self.base_url);
        let request = serde_json::json!({
            "name": model_name
        });
        
        let response = self.client.delete(&url).json(&request).send().await?;
        
        if response.status().is_success() {
            println!("✅ Model {} deleted", model_name);
            Ok(())
        } else {
            Err(anyhow!("Failed to delete model: {}", response.status()))
        }
    }
}

// Enhanced LLM Wrapper that orchestrates all components
pub struct EnhancedLLMWrapper {
    backends: HashMap<String, Box<dyn Backend>>,
    cache_manager: CacheManager,
    template_engine: TemplateEngine,
    streaming_manager: StreamingManager,
    config: EnhancedConfig,
    metrics: MetricsCollector,
    current_backend: String,
}

#[derive(Debug, Clone)]
pub struct MetricsCollector {
    pub requests_total: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub template_renders: u64,
    pub active_streams: u64,
    pub errors_total: u64,
    pub average_response_time_ms: f64,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self {
            requests_total: 0,
            cache_hits: 0,
            cache_misses: 0,
            template_renders: 0,
            active_streams: 0,
            errors_total: 0,
            average_response_time_ms: 0.0,
        }
    }
}

impl MetricsCollector {
    pub fn record_request(&mut self) {
        self.requests_total += 1;
    }

    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    pub fn record_template_render(&mut self) {
        self.template_renders += 1;
    }

    pub fn record_stream_start(&mut self) {
        self.active_streams += 1;
    }

    pub fn record_stream_end(&mut self) {
        if self.active_streams > 0 {
            self.active_streams -= 1;
        }
    }

    pub fn record_error(&mut self) {
        self.errors_total += 1;
    }

    pub fn record_response_time(&mut self, duration_ms: f64) {
        // Simple moving average
        let total_requests = self.requests_total as f64;
        if total_requests > 0.0 {
            self.average_response_time_ms = 
                (self.average_response_time_ms * (total_requests - 1.0) + duration_ms) / total_requests;
        } else {
            self.average_response_time_ms = duration_ms;
        }
    }

    pub fn cache_hit_ratio(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

impl EnhancedLLMWrapper {
    pub async fn new(config: EnhancedConfig) -> Result<Self, WrapperError> {
        // Initialize logging first
        crate::logging::init_logging(&config.logging)
            .map_err(|e| WrapperError::Config(ConfigError::Invalid(format!("Logging init failed: {}", e))))?;
        
        tracing::info!("Initializing EnhancedLLMWrapper with config: {:?}", config);
        // Initialize cache manager
        let cache_manager = if config.cache.enable_persistence {
            CacheManager::new_with_persistence(config.cache.clone()).await?
        } else {
            CacheManager::new(config.cache.clone())
        };

        // Initialize template engine
        let template_config = template::TemplateConfig {
            template_dir: Some(config.templates.template_dir.clone()),
            auto_reload: config.templates.auto_reload,
            enable_sandboxing: true,
            max_template_size: 1024 * 1024,
            max_render_time_ms: 5000,
            allowed_helpers: config.templates.custom_helpers.clone(),
        };
        let template_engine = TemplateEngine::new(template_config);

        // Initialize streaming manager
        let streaming_manager = StreamingManager::new(config.streaming.max_concurrent_streams);

        // Initialize backends
        let mut backends: HashMap<String, Box<dyn Backend>> = HashMap::new();
        
        for (name, backend_config) in &config.backends {
            match backend_config.backend_type {
                config::BackendType::Ollama => {
                    let backend = OllamaBackend::new(backend_config.base_url.clone())?;
                    backends.insert(name.clone(), Box::new(backend));
                }
                config::BackendType::LMStudio => {
                    // TODO: Implement LMStudio backend
                    eprintln!("Warning: LMStudio backend not yet implemented");
                }
                config::BackendType::OpenAI => {
                    // TODO: Implement OpenAI backend
                    eprintln!("Warning: OpenAI backend not yet implemented");
                }
                config::BackendType::Custom => {
                    // TODO: Implement Custom backend
                    eprintln!("Warning: Custom backend not yet implemented");
                }
                config::BackendType::Mock => {
                    let backend = MockBackend::new();
                    backends.insert(name.clone(), Box::new(backend));
                }
            }
        }

        if backends.is_empty() {
            let error = WrapperError::Config(ConfigError::Validation(
                "No valid backends configured".to_string()
            ));
            crate::logging::log_error(&error, "EnhancedLLMWrapper initialization");
            return Err(error);
        }

        let current_backend = backends.keys().next().unwrap().clone();
        
        tracing::info!(
            backends_count = backends.len(),
            current_backend = %current_backend,
            "EnhancedLLMWrapper initialized successfully"
        );

        Ok(Self {
            backends,
            cache_manager,
            template_engine,
            streaming_manager,
            config,
            metrics: MetricsCollector::default(),
            current_backend,
        })
    }

    pub async fn chat_with_template(
        &mut self,
        template_name: &str,
        variables: serde_json::Value,
        model: Option<&str>,
    ) -> Result<StreamResponse, WrapperError> {
        let start_time = std::time::Instant::now();
        self.metrics.record_request();
        
        tracing::info!(
            template_name = template_name,
            model = model,
            "Starting chat with template"
        );

        // Render template with error recovery
        let rendered_prompt = match self.template_engine.render(template_name, &variables) {
            Ok(prompt) => {
                self.metrics.record_template_render();
                crate::logging::log_template_event("render", template_name, true);
                prompt
            }
            Err(e) => {
                self.metrics.record_error();
                crate::logging::log_template_event("render", template_name, false);
                crate::logging::log_error(&e, "Template rendering");
                return Err(WrapperError::Template(e));
            }
        };

        // Create cache key
        let cache_key = cache::CacheKey::new(
            &rendered_prompt,
            model.unwrap_or("default"),
            &std::collections::HashMap::new(),
        );

        // Check cache first with error handling
        match self.cache_manager.get(&cache_key).await {
            Some(cached_response) => {
                self.metrics.record_cache_hit();
                crate::logging::log_cache_event("hit", cache_key.prompt_hash, true);
                
                tracing::debug!("Cache hit for template: {}", template_name);
                
                // Create a mock stream response for cached content
                let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
                let cancellation_token = tokio_util::sync::CancellationToken::new();
                
                // Send the cached response as a single token
                let _ = sender.send(StreamToken {
                    content: cached_response,
                    is_complete: true,
                    metadata: Some(streaming::TokenMetadata {
                        timestamp: chrono::Utc::now(),
                        token_count: None,
                    }),
                });

                return Ok(StreamResponse {
                    id: rand::random(),
                    receiver,
                    cancellation_token,
                });
            }
            None => {
                self.metrics.record_cache_miss();
                crate::logging::log_cache_event("miss", cache_key.prompt_hash, false);
                tracing::debug!("Cache miss for template: {}", template_name);
            }
        }

        // Get backend with error handling
        let backend = self.backends.get(&self.current_backend)
            .ok_or_else(|| {
                let error = WrapperError::Config(ConfigError::Validation(
                    format!("Backend '{}' not found", self.current_backend)
                ));
                crate::logging::log_error(&error, "Backend lookup");
                error
            })?;

        // Create chat request
        let request = streaming::ChatRequest {
            model: model.unwrap_or("default").to_string(),
            messages: vec![streaming::Message {
                role: "user".to_string(),
                content: rendered_prompt.clone(),
                images: None,
            }],
            stream: true,
            options: None,
        };

        // Create stream with error handling and retry logic
        let stream_response = match backend.chat_stream(request).await {
            Ok(response) => {
                self.metrics.record_stream_start();
                crate::logging::log_stream_event("start", response.id, model.unwrap_or("default"));
                response
            }
            Err(e) => {
                self.metrics.record_error();
                crate::logging::log_backend_event("stream_error", &self.current_backend, false, None);
                crate::logging::log_error(&e, "Stream creation");
                return Err(WrapperError::Backend(e));
            }
        };

        // Record response time
        let duration = start_time.elapsed();
        self.metrics.record_response_time(duration.as_millis() as f64);
        crate::logging::log_performance_metric("chat_with_template", duration.as_millis() as f64, true);

        tracing::info!(
            template_name = template_name,
            stream_id = stream_response.id,
            duration_ms = duration.as_millis(),
            "Chat with template completed successfully"
        );

        Ok(stream_response)
    }

    pub async fn chat(
        &mut self,
        message: &str,
        model: Option<&str>,
    ) -> Result<String, WrapperError> {
        let start_time = std::time::Instant::now();
        self.metrics.record_request();

        // Create cache key
        let cache_key = cache::CacheKey::new(
            message,
            model.unwrap_or("default"),
            &std::collections::HashMap::new(),
        );

        // Check cache first
        if let Some(cached_response) = self.cache_manager.get(&cache_key).await {
            self.metrics.record_cache_hit();
            return Ok(cached_response);
        }

        self.metrics.record_cache_miss();

        // Get backend
        let backend = self.backends.get(&self.current_backend)
            .ok_or_else(|| WrapperError::Config(ConfigError::Validation(
                format!("Backend '{}' not found", self.current_backend)
            )))?;

        // Create chat request
        let request = streaming::ChatRequest {
            model: model.unwrap_or("default").to_string(),
            messages: vec![streaming::Message {
                role: "user".to_string(),
                content: message.to_string(),
                images: None,
            }],
            stream: false,
            options: None,
        };

        // Make request
        let response = backend.chat(request).await?;

        // Cache the response
        let metadata = cache::ResponseMetadata {
            model: model.unwrap_or("default").to_string(),
            tokens_used: None,
            response_time: start_time.elapsed(),
            backend_type: backend.backend_type().to_string(),
        };

        self.cache_manager.put(cache_key, response.clone(), metadata).await?;

        // Record response time
        let duration = start_time.elapsed();
        self.metrics.record_response_time(duration.as_millis() as f64);

        Ok(response)
    }

    pub async fn interactive_mode(&mut self) -> Result<(), WrapperError> {
        let mut ui = TerminalUI::new()?;
        
        // Create a channel for streaming tokens
        let (_stream_sender, stream_receiver) = tokio::sync::mpsc::unbounded_channel();
        
        // Update UI with current app state
        let app_state = ui::AppState {
            current_model: self.current_backend.clone(),
            is_streaming: false,
            cache_stats: self.cache_manager.get_stats().clone(),
            active_template: None,
        };
        ui.update_app_state(app_state);

        // Run the UI
        ui.run(stream_receiver).await?;
        
        Ok(())
    }

    pub fn switch_backend(&mut self, backend_name: &str) -> Result<(), WrapperError> {
        if !self.backends.contains_key(backend_name) {
            return Err(WrapperError::Config(ConfigError::Validation(
                format!("Backend '{}' not found", backend_name)
            )));
        }
        
        self.current_backend = backend_name.to_string();
        Ok(())
    }

    pub fn list_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, WrapperError> {
        let backend = self.backends.get(&self.current_backend)
            .ok_or_else(|| WrapperError::Config(ConfigError::Validation(
                format!("Backend '{}' not found", self.current_backend)
            )))?;

        Ok(backend.list_models().await?)
    }

    pub fn get_cache_stats(&self) -> &CacheStats {
        self.cache_manager.get_stats()
    }

    pub fn get_metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    pub fn list_templates(&self) -> Vec<&Template> {
        self.template_engine.list_templates()
    }

    pub async fn save_template(&mut self, template: Template) -> Result<(), WrapperError> {
        self.template_engine.register_template(template)?;
        Ok(())
    }

    pub async fn clear_cache(&mut self) -> Result<(), WrapperError> {
        self.cache_manager.clear();
        Ok(())
    }

    pub async fn invalidate_cache_for_model(&mut self, model: &str) -> Result<(), WrapperError> {
        self.cache_manager.invalidate_model(model);
        Ok(())
    }
}