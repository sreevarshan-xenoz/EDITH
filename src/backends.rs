use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::streaming::{ChatRequest, StreamResponse};
use crate::error::BackendError;

#[derive(Debug, Error)]
pub enum BackendInitError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Authentication failed")]
    Authentication,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendType {
    Ollama,
    LMStudio,
    OpenAI,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: Option<u64>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub capabilities: ModelCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub supports_vision: bool,
    pub supports_thinking: bool,
    pub supports_streaming: bool,
    pub max_tokens: Option<u32>,
    pub context_length: Option<u32>,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            supports_vision: false,
            supports_thinking: false,
            supports_streaming: true,
            max_tokens: Some(4096),
            context_length: Some(4096),
        }
    }
}

#[async_trait]
pub trait Backend: Send + Sync {
    /// Send a chat request and get a complete response
    async fn chat(&self, request: ChatRequest) -> Result<String, BackendError>;
    
    /// Send a chat request and get a streaming response
    async fn chat_stream(&self, request: ChatRequest) -> Result<StreamResponse, BackendError>;
    
    /// List available models
    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError>;
    
    /// Get model capabilities
    async fn get_model_capabilities(&self, model_name: &str) -> Result<ModelCapabilities, BackendError>;
    
    /// Get backend capabilities
    fn capabilities(&self) -> &BackendCapabilities;
    
    /// Get backend type
    fn backend_type(&self) -> BackendType;
    
    /// Health check
    async fn health_check(&self) -> Result<(), BackendError>;
}

#[derive(Debug, Clone)]
pub struct BackendCapabilities {
    pub supports_streaming: bool,
    pub supports_vision: bool,
    pub supports_thinking: bool,
    pub max_concurrent_requests: usize,
}

impl Default for BackendCapabilities {
    fn default() -> Self {
        Self {
            supports_streaming: true,
            supports_vision: false,
            supports_thinking: false,
            max_concurrent_requests: 10,
        }
    }
}

/// Ollama backend implementation
pub struct OllamaBackend {
    client: reqwest::Client,
    base_url: String,
    capabilities: BackendCapabilities,
    streaming_manager: crate::streaming::StreamingManager,
}

impl OllamaBackend {
    pub fn new(base_url: String) -> Result<Self, BackendInitError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| BackendInitError::Connection(e.to_string()))?;

        let streaming_manager = crate::streaming::StreamingManager::new(10);

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            capabilities: BackendCapabilities::default(),
            streaming_manager,
        })
    }

    async fn detect_capabilities(&mut self) -> Result<(), BackendError> {
        // Try to get model list to verify connection
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(BackendError::Connection(format!(
                "Failed to connect to Ollama: {}",
                response.status()
            )));
        }

        // Ollama supports streaming by default
        self.capabilities.supports_streaming = true;
        
        Ok(())
    }
}

#[async_trait]
impl Backend for OllamaBackend {
    async fn chat(&self, request: ChatRequest) -> Result<String, BackendError> {
        let url = format!("{}/api/chat", self.base_url);
        
        // Convert to non-streaming request
        let mut ollama_request = request;
        ollama_request.stream = false;
        
        let response = self.client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(BackendError::Connection(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let chat_response: serde_json::Value = response.json().await?;
        
        if let Some(content) = chat_response.get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str()) 
        {
            Ok(content.to_string())
        } else {
            Err(BackendError::InvalidResponse)
        }
    }

    async fn chat_stream(&self, _request: ChatRequest) -> Result<StreamResponse, BackendError> {
        // This would need to be implemented with proper streaming manager integration
        // For now, return an error indicating it's not implemented
        Err(BackendError::Connection("Streaming not yet integrated".to_string()))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(BackendError::Connection(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let models_response: serde_json::Value = response.json().await?;
        
        if let Some(models_array) = models_response.get("models").and_then(|m| m.as_array()) {
            let models = models_array
                .iter()
                .filter_map(|model| {
                    let name = model.get("name")?.as_str()?.to_string();
                    let size = model.get("size").and_then(|s| s.as_u64());
                    
                    Some(ModelInfo {
                        name: name.clone(),
                        size,
                        modified_at: None, // Ollama doesn't provide this in a standard format
                        capabilities: self.detect_model_capabilities(&name),
                    })
                })
                .collect();
            
            Ok(models)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_model_capabilities(&self, model_name: &str) -> Result<ModelCapabilities, BackendError> {
        Ok(self.detect_model_capabilities(model_name))
    }

    fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Ollama
    }

    async fn health_check(&self) -> Result<(), BackendError> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            Err(BackendError::Connection(format!(
                "Health check failed: {}",
                response.status()
            )))
        }
    }
}

impl OllamaBackend {
    fn detect_model_capabilities(&self, model_name: &str) -> ModelCapabilities {
        let model_lower = model_name.to_lowercase();
        
        let supports_vision = model_lower.contains("llava") 
            || model_lower.contains("vision")
            || model_lower.contains("bakllava")
            || model_lower.contains("moondream");
            
        let supports_thinking = model_lower.contains("o1")
            || model_lower.contains("reasoning")
            || model_lower.contains("thinking");

        ModelCapabilities {
            supports_vision,
            supports_thinking,
            supports_streaming: true, // Ollama supports streaming for all models
            max_tokens: Some(4096),
            context_length: Some(4096),
        }
    }
}

/// Mock backend for testing
pub struct MockBackend {
    capabilities: BackendCapabilities,
    responses: HashMap<String, String>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            capabilities: BackendCapabilities::default(),
            responses: HashMap::new(),
        }
    }

    pub fn add_response(&mut self, prompt: String, response: String) {
        self.responses.insert(prompt, response);
    }
}

#[async_trait]
impl Backend for MockBackend {
    async fn chat(&self, request: ChatRequest) -> Result<String, BackendError> {
        // Simple mock: return first message content as key
        if let Some(message) = request.messages.first() {
            if let Some(response) = self.responses.get(&message.content) {
                Ok(response.clone())
            } else {
                Ok("Mock response".to_string())
            }
        } else {
            Err(BackendError::InvalidResponse)
        }
    }

    async fn chat_stream(&self, _request: ChatRequest) -> Result<StreamResponse, BackendError> {
        Err(BackendError::Connection("Mock streaming not implemented".to_string()))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, BackendError> {
        Ok(vec![
            ModelInfo {
                name: "mock-model".to_string(),
                size: Some(1024),
                modified_at: Some(chrono::Utc::now()),
                capabilities: ModelCapabilities::default(),
            }
        ])
    }

    async fn get_model_capabilities(&self, _model_name: &str) -> Result<ModelCapabilities, BackendError> {
        Ok(ModelCapabilities::default())
    }

    fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Custom
    }

    async fn health_check(&self) -> Result<(), BackendError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_backend() {
        let mut backend = MockBackend::new();
        backend.add_response("Hello".to_string(), "Hi there!".to_string());

        let request = ChatRequest {
            model: "test".to_string(),
            messages: vec![crate::streaming::Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
                images: None,
            }],
            stream: false,
            options: None,
        };

        let response = backend.chat(request).await.unwrap();
        assert_eq!(response, "Hi there!");
    }

    #[tokio::test]
    async fn test_mock_backend_health_check() {
        let backend = MockBackend::new();
        assert!(backend.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_backend_list_models() {
        let backend = MockBackend::new();
        let models = backend.list_models().await.unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "mock-model");
    }
}