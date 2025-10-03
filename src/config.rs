use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::cache::CacheConfig;
use crate::error::ConfigError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedConfig {
    pub backends: HashMap<String, BackendConfig>,
    pub cache: CacheConfig,
    pub ui: UIConfig,
    pub templates: TemplateConfig,
    pub logging: LoggingConfig,
    pub streaming: StreamingConfig,
}

impl Default for EnhancedConfig {
    fn default() -> Self {
        let mut backends = HashMap::new();
        backends.insert("ollama".to_string(), BackendConfig::default());
        
        Self {
            backends,
            cache: CacheConfig::default(),
            ui: UIConfig::default(),
            templates: TemplateConfig::default(),
            logging: LoggingConfig::default(),
            streaming: StreamingConfig::default(),
        }
    }
}

impl EnhancedConfig {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(&path)
            .map_err(|_| ConfigError::FileNotFound(path.as_ref().display().to_string()))?;
        
        let config: EnhancedConfig = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        
        config.validate()?;
        Ok(config)
    }

    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        
        std::fs::write(path, content)?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.backends.is_empty() {
            return Err(ConfigError::Invalid("At least one backend must be configured".to_string()));
        }

        for (name, backend) in &self.backends {
            if backend.base_url.is_empty() {
                return Err(ConfigError::Invalid(format!("Backend '{}' must have a base_url", name)));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub backend_type: BackendType,
    pub base_url: String,
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    pub retry_attempts: u32,
    pub rate_limit: Option<RateLimit>,
    pub default_model: Option<String>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            backend_type: BackendType::Ollama,
            base_url: "http://localhost:11434".to_string(),
            timeout: Duration::from_secs(30),
            retry_attempts: 3,
            rate_limit: Some(RateLimit::default()),
            default_model: Some("llama3.2".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendType {
    Ollama,
    LMStudio,
    OpenAI,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub max_concurrent: usize,
    pub requests_per_minute: u32,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            requests_per_minute: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    pub theme: String,
    pub syntax_highlighting: bool,
    pub auto_scroll: bool,
    pub max_history: usize,
    pub show_timestamps: bool,
    pub show_model_info: bool,
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            syntax_highlighting: true,
            auto_scroll: true,
            max_history: 1000,
            show_timestamps: true,
            show_model_info: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub template_dir: PathBuf,
    pub auto_reload: bool,
    pub custom_helpers: Vec<String>,
    pub default_template: Option<String>,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            template_dir: PathBuf::from("templates"),
            auto_reload: true,
            custom_helpers: Vec::new(),
            default_template: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub file: Option<PathBuf>,
    pub structured: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            file: None,
            structured: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub max_concurrent_streams: usize,
    pub buffer_size: usize,
    pub enable_cancellation: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 10,
            buffer_size: 8192,
            enable_cancellation: true,
        }
    }
}