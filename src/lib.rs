use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use futures_util::StreamExt;
use mime_guess::from_path;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub model_name: String,
    pub supports_vision: bool,
    pub supports_thinking: bool,
    pub supports_streaming: bool,
    pub max_tokens: u32,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            model_name: String::new(),
            supports_vision: false,
            supports_thinking: false,
            supports_streaming: true,
            max_tokens: 4096,
        }
    }
}

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
struct ModelInfo {
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
    capabilities: ModelCapabilities,
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
            capabilities: ModelCapabilities::default(),
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
            let model_info: ModelInfo = response.json().await?;
            
            if let Some(current_model) = model_info.models.iter().find(|m| m.name.contains(&self.model)) {
                self.capabilities.model_name = current_model.name.clone();
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
    
    pub fn capabilities(&self) -> &ModelCapabilities {
        &self.capabilities
    }
    
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        if response.status().is_success() {
            let model_info: ModelInfo = response.json().await?;
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