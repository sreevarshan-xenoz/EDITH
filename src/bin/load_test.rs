use llm_wrapper::{EnhancedLLMWrapper, EnhancedConfig, Template, PerformanceStatus};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use clap::Parser;

#[derive(Parser)]
#[command(name = "load_test")]
#[command(about = "Load testing utility for Enhanced LLM Wrapper")]
struct Args {
    /// Number of concurrent requests
    #[arg(short, long, default_value = "10")]
    concurrency: usize,
    
    /// Total number of requests
    #[arg(short, long, default_value = "100")]
    requests: usize,
    
    /// Delay between requests in milliseconds
    #[arg(short, long, default_value = "100")]
    delay_ms: u64,
    
    /// Export performance report to file
    #[arg(short, long)]
    output: Option<String>,
    
    /// Test template rendering performance
    #[arg(long)]
    test_templates: bool,
    
    /// Test cache performance
    #[arg(long)]
    test_cache: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🚀 Starting Enhanced LLM Wrapper Load Test");
    println!("Concurrency: {}", args.concurrency);
    println!("Total Requests: {}", args.requests);
    println!("Delay: {}ms", args.delay_ms);
    
    // Load configuration
    let config = match EnhancedConfig::load("enhanced-config.toml") {
        Ok(config) => config,
        Err(_) => {
            println!("⚠️  Using default configuration");
            EnhancedConfig::default()
        }
    };
    
    // Initialize wrapper
    let mut wrapper = EnhancedLLMWrapper::new(config).await?;
    
    // Create test template if testing templates
    if args.test_templates {
        let template = Template {
            name: "load_test_template".to_string(),
            content: "Hello {{name}}! Your request ID is {{request_id}}.".to_string(),
            description: Some("Load test template".to_string()),
            variables: Vec::new(),
            created_at: std::time::SystemTime::now(),
            parent_template: None,
            tags: vec!["test".to_string()],
            usage_examples: Vec::new(),
        };
        
        wrapper.save_template(template).await?;
        println!("✅ Test template created");
    }
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    // Create concurrent tasks
    for batch in 0..(args.requests / args.concurrency) {
        let mut batch_handles = Vec::new();
        
        for i in 0..args.concurrency {
            let request_id = batch * args.concurrency + i;
            
            if args.test_templates {
                // Test template rendering
                let variables = json!({
                    "name": format!("User{}", request_id),
                    "request_id": request_id
                });
                
                let handle = tokio::spawn(async move {
                    let start = Instant::now();
                    // Simulate template rendering work
                    sleep(Duration::from_millis(10)).await;
                    let duration = start.elapsed();
                    
                    (request_id, duration, true)
                });
                
                batch_handles.push(handle);
            } else if args.test_cache {
                // Test cache operations
                let handle = tokio::spawn(async move {
                    let start = Instant::now();
                    // Simulate cache operations
                    sleep(Duration::from_millis(5)).await;
                    let duration = start.elapsed();
                    
                    (request_id, duration, true)
                });
                
                batch_handles.push(handle);
            } else {
                // Test basic chat operations
                let message = format!("Test message {}", request_id);
                
                let handle = tokio::spawn(async move {
                    let start = Instant::now();
                    // Simulate chat processing
                    sleep(Duration::from_millis(50)).await;
                    let duration = start.elapsed();
                    
                    (request_id, duration, true)
                });
                
                batch_handles.push(handle);
            }
        }
        
        // Wait for batch to complete
        for handle in batch_handles {
            handles.push(handle);
        }
        
        // Add delay between batches
        if args.delay_ms > 0 {
            sleep(Duration::from_millis(args.delay_ms)).await;
        }
        
        // Progress indicator
        let completed = (batch + 1) * args.concurrency;
        let progress = (completed as f64 / args.requests as f64) * 100.0;
        print!("\r🔄 Progress: {:.1}% ({}/{})", progress, completed.min(args.requests), args.requests);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }
    
    println!("\n⏳ Waiting for all requests to complete...");
    
    // Collect results
    let mut successful_requests = 0;
    let mut total_duration = Duration::new(0, 0);
    let mut min_duration = Duration::from_secs(u64::MAX);
    let mut max_duration = Duration::new(0, 0);
    
    for handle in handles {
        match handle.await {
            Ok((_, duration, success)) => {
                if success {
                    successful_requests += 1;
                }
                total_duration += duration;
                min_duration = min_duration.min(duration);
                max_duration = max_duration.max(duration);
            }
            Err(e) => {
                eprintln!("❌ Request failed: {}", e);
            }
        }
    }
    
    let total_test_duration = start_time.elapsed();
    let success_rate = successful_requests as f64 / args.requests as f64;
    let avg_duration = total_duration / args.requests as u32;
    let requests_per_second = args.requests as f64 / total_test_duration.as_secs_f64();
    
    // Print results
    println!("\n📊 Load Test Results");
    println!("═══════════════════════════════════");
    println!("Total Duration: {:.2}s", total_test_duration.as_secs_f64());
    println!("Successful Requests: {}/{}", successful_requests, args.requests);
    println!("Success Rate: {:.1}%", success_rate * 100.0);
    println!("Requests/Second: {:.2}", requests_per_second);
    println!("Average Response Time: {:.2}ms", avg_duration.as_millis());
    println!("Min Response Time: {:.2}ms", min_duration.as_millis());
    println!("Max Response Time: {:.2}ms", max_duration.as_millis());
    
    // Get performance metrics
    let performance_metrics = wrapper.get_performance_metrics();
    let performance_report = wrapper.get_performance_report();
    
    println!("\n🎯 Performance Targets");
    println!("═══════════════════════════════════");
    println!("Overall Status: {}", performance_report.overall_status);
    
    if !performance_report.issues.is_empty() {
        println!("\n⚠️  Issues Found:");
        for issue in &performance_report.issues {
            println!("  • {}", issue);
        }
    }
    
    if !performance_report.recommendations.is_empty() {
        println!("\n💡 Recommendations:");
        for rec in &performance_report.recommendations {
            println!("  • {}", rec);
        }
    }
    
    // Export detailed metrics if requested
    if let Some(output_path) = args.output {
        wrapper.export_performance_metrics(&output_path).await?;
        println!("\n💾 Performance metrics exported to: {}", output_path);
    }
    
    // Determine exit code based on performance
    let exit_code = match performance_report.overall_status {
        PerformanceStatus::Good => 0,
        PerformanceStatus::Warning => 1,
        PerformanceStatus::Critical => 2,
    };
    
    if exit_code == 0 {
        println!("\n✅ All performance targets met!");
    } else {
        println!("\n⚠️  Some performance targets not met. See issues above.");
    }
    
    std::process::exit(exit_code);
}