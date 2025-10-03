use thiserror::Error;

#[derive(Debug, Error)]
pub enum WrapperError {
    #[error("Backend error: {0}")]
    Backend(#[from] BackendError),
    
    #[error("Cache error: {0}")]
    Cache(#[from] crate::cache::CacheError),
    
    #[error("Template error: {0}")]
    Template(#[from] crate::template::TemplateError),
    
    #[error("UI error: {0}")]
    UI(#[from] crate::ui::UIError),
    
    #[error("Stream error: {0}")]
    Stream(#[from] crate::streaming::StreamError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Connection failed: {0}")]
    Connection(String),
    
    #[error("Authentication failed")]
    Authentication,
    
    #[error("Rate limit exceeded")]
    RateLimit,
    
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    
    #[error("Request timeout")]
    Timeout,
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Invalid response format")]
    InvalidResponse,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}