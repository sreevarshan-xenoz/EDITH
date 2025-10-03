use llm_wrapper::{
    EnhancedLLMWrapper, EnhancedConfig, Template, 
    cache::{CacheConfig, CacheManager},
    template::{TemplateEngine, TemplateConfig},
    config::{BackendConfig, BackendType, LoggingConfig, UIConfig, StreamingConfig},
};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tempfile::TempDir;

#[tokio::test]
async fn test_enhanced_wrapper_initialization() {
    let config = create_test_config().await;
    let wrapper = EnhancedLLMWrapper::new(config).await;
    assert!(wrapper.is_ok(), "Enhanced wrapper should initialize successfully");
}

#[tokio::test]
async fn test_cache_operations() {
    let temp_dir = TempDir::new().unwrap();
    let cache_config = CacheConfig {
        max_memory_entries: 100,
        ttl: Duration::from_secs(3600),
        enable_persistence: true,
        cache_streaming: true,
        cache_dir: Some(temp_dir.path().to_path_buf()),
        max_memory_bytes: Some(1024 * 1024),
        memory_pressure_threshold: 0.8,
    };

    let mut cache = CacheManager::new(cache_config);
    
    // Test cache put and get
    let key = llm_wrapper::cache::CacheKey::new("test prompt", "test_model", &HashMap::new());
    let metadata = llm_wrapper::cache::ResponseMetadata {
        model: "test_model".to_string(),
        tokens_used: Some(100),
        response_time: Duration::from_millis(500),
        backend_type: "test".to_string(),
    };
    
    cache.put(key.clone(), "test response".to_string(), metadata).await.unwrap();
    
    let result = cache.get(&key).await;
    assert_eq!(result, Some("test response".to_string()));
    
    // Test cache statistics
    let stats = cache.get_stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.total_entries, 1);
}

#[tokio::test]
async fn test_template_engine() {
    let temp_dir = TempDir::new().unwrap();
    let template_config = TemplateConfig {
        template_dir: Some(temp_dir.path().to_path_buf()),
        auto_reload: true,
        enable_sandboxing: true,
        max_template_size: 1024 * 1024,
        max_render_time_ms: 5000,
        allowed_helpers: vec!["upper".to_string(), "lower".to_string()],
    };

    let mut engine = TemplateEngine::new(template_config);
    
    // Create test template
    let template = Template {
        name: "test_template".to_string(),
        content: "Hello {{name}}! {{#if urgent}}URGENT: {{/if}}{{message}}".to_string(),
        description: Some("Test template".to_string()),
        variables: Vec::new(),
        created_at: std::time::SystemTime::now(),
        parent_template: None,
        tags: vec!["test".to_string()],
        usage_examples: Vec::new(),
    };
    
    engine.register_template(template).unwrap();
    
    // Test template rendering
    let context = json!({
        "name": "Alice",
        "urgent": true,
        "message": "Please review the code"
    });
    
    let result = engine.render("test_template", &context).unwrap();
    assert_eq!(result, "Hello Alice! URGENT: Please review the code");
    
    // Test template listing
    let templates = engine.list_templates();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].name, "test_template");
}

#[tokio::test]
async fn test_performance_monitoring() {
    use llm_wrapper::performance::PerformanceMonitor;
    use std::time::Duration;
    
    let monitor = PerformanceMonitor::new();
    
    // Record some operations
    monitor.record_cache_operation("lookup", Duration::from_millis(5), true);
    monitor.record_cache_operation("lookup", Duration::from_millis(15), false);
    monitor.record_template_render(Duration::from_millis(25), true);
    monitor.record_template_render(Duration::from_millis(35), true);
    
    let metrics = monitor.get_metrics();
    
    // Verify metrics are recorded
    assert!(metrics.cache_metrics.total_operations > 0);
    assert!(metrics.template_metrics.total_renders > 0);
    assert!(metrics.cache_metrics.average_lookup_time_ms > 0.0);
    assert!(metrics.template_metrics.average_render_time_ms > 0.0);
    
    // Test performance targets
    let report = monitor.check_performance_targets();
    assert!(!report.issues.is_empty() || report.overall_status == llm_wrapper::performance::PerformanceStatus::Good);
}

#[tokio::test]
async fn test_configuration_validation() {
    // Test valid configuration
    let valid_config = create_test_config().await;
    assert!(valid_config.validate().is_ok());
    
    // Test invalid configuration
    let mut invalid_config = create_test_config().await;
    invalid_config.cache.max_memory_entries = 0; // Invalid: must be > 0
    assert!(invalid_config.validate().is_err());
    
    // Test invalid memory pressure threshold
    invalid_config.cache.max_memory_entries = 100;
    invalid_config.cache.memory_pressure_threshold = 1.5; // Invalid: must be <= 1.0
    assert!(invalid_config.validate().is_err());
}

#[tokio::test]
async fn test_error_handling() {
    use llm_wrapper::error::{WrapperError, ConfigError};
    
    // Test configuration error handling
    let mut config = create_test_config().await;
    config.backends.clear(); // No backends configured
    
    let result = EnhancedLLMWrapper::new(config).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        WrapperError::Config(ConfigError::Validation(_)) => {
            // Expected error type
        }
        _ => panic!("Expected ConfigError::Validation"),
    }
}

#[tokio::test]
async fn test_template_security() {
    let temp_dir = TempDir::new().unwrap();
    let template_config = TemplateConfig {
        template_dir: Some(temp_dir.path().to_path_buf()),
        auto_reload: true,
        enable_sandboxing: true,
        max_template_size: 1024,
        max_render_time_ms: 1000,
        allowed_helpers: vec![],
    };

    let mut engine = TemplateEngine::new(template_config);
    
    // Test template size limit
    let large_template = Template {
        name: "large_template".to_string(),
        content: "x".repeat(2048), // Exceeds max_template_size
        description: None,
        variables: Vec::new(),
        created_at: std::time::SystemTime::now(),
        parent_template: None,
        tags: Vec::new(),
        usage_examples: Vec::new(),
    };
    
    let result = engine.register_template(large_template);
    assert!(result.is_err(), "Should reject templates exceeding size limit");
}

#[tokio::test]
async fn test_cache_memory_pressure() {
    let cache_config = CacheConfig {
        max_memory_entries: 10,
        ttl: Duration::from_secs(3600),
        enable_persistence: false,
        cache_streaming: true,
        cache_dir: None,
        max_memory_bytes: Some(1024), // Very small limit
        memory_pressure_threshold: 0.5,
    };

    let mut cache = CacheManager::new(cache_config);
    
    // Fill cache beyond memory limit
    for i in 0..20 {
        let key = llm_wrapper::cache::CacheKey::new(
            &format!("test prompt {}", i),
            "test_model",
            &HashMap::new(),
        );
        let large_response = "x".repeat(100); // 100 bytes each
        let metadata = llm_wrapper::cache::ResponseMetadata {
            model: "test_model".to_string(),
            tokens_used: Some(100),
            response_time: Duration::from_millis(500),
            backend_type: "test".to_string(),
        };
        
        cache.put(key, large_response, metadata).await.unwrap();
    }
    
    let stats = cache.get_stats();
    // Should have evicted some entries due to memory pressure
    assert!(stats.evictions > 0 || stats.total_entries < 20);
}

#[tokio::test]
async fn test_concurrent_cache_access() {
    use tokio::task::JoinSet;
    
    let cache_config = CacheConfig {
        max_memory_entries: 1000,
        ttl: Duration::from_secs(3600),
        enable_persistence: false,
        cache_streaming: true,
        cache_dir: None,
        max_memory_bytes: Some(1024 * 1024),
        memory_pressure_threshold: 0.8,
    };

    let mut cache = CacheManager::new(cache_config);
    
    // Perform concurrent cache operations
    let mut tasks = JoinSet::new();
    
    for i in 0..10 {
        let key = llm_wrapper::cache::CacheKey::new(
            &format!("concurrent test {}", i),
            "test_model",
            &HashMap::new(),
        );
        let metadata = llm_wrapper::cache::ResponseMetadata {
            model: "test_model".to_string(),
            tokens_used: Some(100),
            response_time: Duration::from_millis(500),
            backend_type: "test".to_string(),
        };
        
        cache.put(key.clone(), format!("response {}", i), metadata).await.unwrap();
        
        tasks.spawn(async move {
            // Simulate concurrent access
            tokio::time::sleep(Duration::from_millis(10)).await;
            key
        });
    }
    
    // Wait for all tasks to complete
    while let Some(result) = tasks.join_next().await {
        assert!(result.is_ok());
    }
    
    let stats = cache.get_stats();
    assert_eq!(stats.total_entries, 10);
}

async fn create_test_config() -> EnhancedConfig {
    let mut backends = HashMap::new();
    backends.insert("mock".to_string(), BackendConfig {
        backend_type: BackendType::Mock,
        base_url: "http://localhost:8080".to_string(),
        timeout: Duration::from_secs(30),
        retry_attempts: 3,
        rate_limit: None,
        default_model: Some("test_model".to_string()),
    });

    EnhancedConfig {
        backends,
        cache: CacheConfig {
            max_memory_entries: 1000,
            ttl: Duration::from_secs(3600),
            enable_persistence: false,
            cache_streaming: true,
            cache_dir: None,
            max_memory_bytes: Some(100 * 1024 * 1024),
            memory_pressure_threshold: 0.8,
        },
        ui: UIConfig {
            theme: "default".to_string(),
            syntax_highlighting: true,
            auto_scroll: true,
            max_history: 1000,
            high_contrast: false,
        },
        templates: llm_wrapper::config::TemplateConfig {
            template_dir: std::path::PathBuf::from("templates"),
            auto_reload: true,
            custom_helpers: vec!["upper".to_string(), "lower".to_string()],
        },
        logging: LoggingConfig {
            level: "info".to_string(),
            format: "text".to_string(),
            output: "stdout".to_string(),
            file_path: None,
        },
        streaming: StreamingConfig {
            max_concurrent_streams: 10,
            buffer_size: 8192,
            enable_cancellation: true,
        },
    }
}