use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use llm_wrapper::{
    EnhancedLLMWrapper, EnhancedConfig, Template, CacheManager, TemplateEngine,
    cache::{CacheKey, CacheConfig, ResponseMetadata},
    template::TemplateConfig,
    streaming::{StreamingManager, ChatRequest, Message},
};
use serde_json::json;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

fn benchmark_cache_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("cache_operations");
    
    // Test different cache sizes
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("cache_put_get", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let config = CacheConfig {
                        max_memory_entries: size,
                        ttl: Duration::from_secs(3600),
                        enable_persistence: false,
                        cache_streaming: true,
                        cache_dir: None,
                        max_memory_bytes: Some(100 * 1024 * 1024),
                        memory_pressure_threshold: 0.8,
                    };
                    
                    let mut cache = CacheManager::new(config);
                    
                    // Benchmark cache put and get operations
                    let start = Instant::now();
                    
                    for i in 0..100 {
                        let key = CacheKey::new(
                            &format!("test_prompt_{}", i),
                            "test_model",
                            &HashMap::new(),
                        );
                        
                        let metadata = ResponseMetadata {
                            model: "test_model".to_string(),
                            tokens_used: Some(100),
                            response_time: Duration::from_millis(500),
                            backend_type: "test".to_string(),
                        };
                        
                        cache.put(key.clone(), format!("response_{}", i), metadata).await.unwrap();
                        
                        // Immediately get it back
                        let result = cache.get(&key).await;
                        black_box(result);
                    }
                    
                    start.elapsed()
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_template_rendering(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("template_rendering");
    
    // Test different template complexities
    let templates = vec![
        ("simple", "Hello {{name}}!"),
        ("medium", "Hello {{name}}! You have {{#if messages}}{{messages.length}}{{else}}no{{/if}} messages."),
        ("complex", r#"
            {{#each users}}
            User: {{name}} ({{email}})
            {{#if active}}
                Status: Active
                {{#each permissions}}
                - {{this}}
                {{/each}}
            {{else}}
                Status: Inactive
            {{/if}}
            {{/each}}
        "#),
    ];
    
    for (name, template_content) in templates {
        group.bench_function(name, |b| {
            b.to_async(&rt).iter(|| async {
                let config = TemplateConfig {
                    template_dir: None,
                    auto_reload: false,
                    enable_sandboxing: true,
                    max_template_size: 1024 * 1024,
                    max_render_time_ms: 5000,
                    allowed_helpers: vec!["upper".to_string(), "lower".to_string()],
                };
                
                let mut engine = TemplateEngine::new(config);
                
                let template = Template {
                    name: "test_template".to_string(),
                    content: template_content.to_string(),
                    description: Some("Test template".to_string()),
                    variables: Vec::new(),
                    created_at: std::time::SystemTime::now(),
                    parent_template: None,
                    tags: Vec::new(),
                    usage_examples: Vec::new(),
                };
                
                engine.register_template(template).unwrap();
                
                let context = match name {
                    "simple" => json!({"name": "World"}),
                    "medium" => json!({"name": "Alice", "messages": [1, 2, 3]}),
                    "complex" => json!({
                        "users": [
                            {
                                "name": "Alice",
                                "email": "alice@example.com",
                                "active": true,
                                "permissions": ["read", "write"]
                            },
                            {
                                "name": "Bob",
                                "email": "bob@example.com",
                                "active": false,
                                "permissions": []
                            }
                        ]
                    }),
                    _ => json!({}),
                };
                
                let start = Instant::now();
                let result = engine.render("test_template", &context);
                black_box(result);
                start.elapsed()
            });
        });
    }
    
    group.finish();
}

fn benchmark_streaming_manager(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("streaming_manager");
    
    group.bench_function("create_multiple_streams", |b| {
        b.to_async(&rt).iter(|| async {
            let mut manager = StreamingManager::new(10);
            
            let start = Instant::now();
            
            // Create multiple concurrent streams (mock)
            let mut handles = Vec::new();
            
            for i in 0..5 {
                let request = ChatRequest {
                    model: "test_model".to_string(),
                    messages: vec![Message {
                        role: "user".to_string(),
                        content: format!("Test message {}", i),
                        images: None,
                    }],
                    stream: true,
                    options: None,
                };
                
                // Note: This would normally create real streams, but for benchmarking
                // we're just measuring the setup overhead
                black_box(request);
            }
            
            start.elapsed()
        });
    });
    
    group.finish();
}

fn benchmark_end_to_end_workflow(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("end_to_end_template_cache", |b| {
        b.to_async(&rt).iter(|| async {
            // Create a minimal enhanced config for testing
            let config = EnhancedConfig::default();
            
            // This benchmark would measure the full workflow:
            // 1. Template rendering
            // 2. Cache lookup
            // 3. Response generation (mocked)
            // 4. Cache storage
            
            let start = Instant::now();
            
            // Simulate the workflow without actual network calls
            let template_name = "test_template";
            let variables = json!({"name": "benchmark_user"});
            
            // Mock template rendering time
            tokio::time::sleep(Duration::from_micros(50)).await;
            
            // Mock cache operations
            tokio::time::sleep(Duration::from_micros(10)).await;
            
            // Mock response generation
            tokio::time::sleep(Duration::from_micros(200)).await;
            
            black_box((template_name, variables));
            start.elapsed()
        });
    });
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_usage");
    
    group.bench_function("cache_memory_pressure", |b| {
        b.to_async(&rt).iter(|| async {
            let config = CacheConfig {
                max_memory_entries: 1000,
                ttl: Duration::from_secs(3600),
                enable_persistence: false,
                cache_streaming: true,
                cache_dir: None,
                max_memory_bytes: Some(1024 * 1024), // 1MB limit
                memory_pressure_threshold: 0.8,
            };
            
            let mut cache = CacheManager::new(config);
            
            let start = Instant::now();
            
            // Fill cache beyond memory pressure threshold
            for i in 0..2000 {
                let key = CacheKey::new(
                    &format!("large_prompt_{}", i),
                    "test_model",
                    &HashMap::new(),
                );
                
                let large_response = "x".repeat(1024); // 1KB response
                
                let metadata = ResponseMetadata {
                    model: "test_model".to_string(),
                    tokens_used: Some(100),
                    response_time: Duration::from_millis(500),
                    backend_type: "test".to_string(),
                };
                
                cache.put(key, large_response, metadata).await.unwrap();
            }
            
            start.elapsed()
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_cache_operations,
    benchmark_template_rendering,
    benchmark_streaming_manager,
    benchmark_end_to_end_workflow,
    benchmark_memory_usage
);

criterion_main!(benches);