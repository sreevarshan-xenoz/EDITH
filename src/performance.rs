use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use tokio::time::interval;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub cache_metrics: CachePerformanceMetrics,
    pub template_metrics: TemplatePerformanceMetrics,
    pub streaming_metrics: StreamingPerformanceMetrics,
    pub system_metrics: SystemPerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePerformanceMetrics {
    pub hit_ratio: f64,
    pub average_lookup_time_ms: f64,
    pub average_store_time_ms: f64,
    pub memory_usage_bytes: usize,
    pub eviction_rate: f64,
    pub total_operations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatePerformanceMetrics {
    pub average_render_time_ms: f64,
    pub render_success_rate: f64,
    pub total_renders: u64,
    pub cache_hit_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingPerformanceMetrics {
    pub average_first_token_time_ms: f64,
    pub average_tokens_per_second: f64,
    pub active_streams: u64,
    pub total_streams_created: u64,
    pub stream_success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPerformanceMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub error_rate: f64,
}

#[derive(Debug)]
pub struct PerformanceMonitor {
    metrics: Arc<Mutex<PerformanceMetrics>>,
    start_time: Instant,
    operation_times: Arc<Mutex<HashMap<String, Vec<Duration>>>>,
    counters: Arc<Mutex<HashMap<String, u64>>>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            cache_metrics: CachePerformanceMetrics {
                hit_ratio: 0.0,
                average_lookup_time_ms: 0.0,
                average_store_time_ms: 0.0,
                memory_usage_bytes: 0,
                eviction_rate: 0.0,
                total_operations: 0,
            },
            template_metrics: TemplatePerformanceMetrics {
                average_render_time_ms: 0.0,
                render_success_rate: 0.0,
                total_renders: 0,
                cache_hit_ratio: 0.0,
            },
            streaming_metrics: StreamingPerformanceMetrics {
                average_first_token_time_ms: 0.0,
                average_tokens_per_second: 0.0,
                active_streams: 0,
                total_streams_created: 0,
                stream_success_rate: 0.0,
            },
            system_metrics: SystemPerformanceMetrics {
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
                uptime_seconds: 0,
                total_requests: 0,
                error_rate: 0.0,
            },
        }
    }
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
            start_time: Instant::now(),
            operation_times: Arc::new(Mutex::new(HashMap::new())),
            counters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn record_operation_time(&self, operation: &str, duration: Duration) {
        let mut times = self.operation_times.lock().unwrap();
        times.entry(operation.to_string()).or_insert_with(Vec::new).push(duration);
        
        // Keep only the last 1000 measurements to prevent memory growth
        if let Some(vec) = times.get_mut(operation) {
            if vec.len() > 1000 {
                vec.drain(0..vec.len() - 1000);
            }
        }
    }

    pub fn increment_counter(&self, counter: &str) {
        let mut counters = self.counters.lock().unwrap();
        *counters.entry(counter.to_string()).or_insert(0) += 1;
    }

    pub fn record_cache_operation(&self, operation_type: &str, duration: Duration, success: bool) {
        self.record_operation_time(&format!("cache_{}", operation_type), duration);
        if success {
            self.increment_counter(&format!("cache_{}_success", operation_type));
        } else {
            self.increment_counter(&format!("cache_{}_error", operation_type));
        }
    }

    pub fn record_template_render(&self, duration: Duration, success: bool) {
        self.record_operation_time("template_render", duration);
        if success {
            self.increment_counter("template_render_success");
        } else {
            self.increment_counter("template_render_error");
        }
    }

    pub fn record_stream_operation(&self, operation_type: &str, duration: Option<Duration>) {
        if let Some(d) = duration {
            self.record_operation_time(&format!("stream_{}", operation_type), d);
        }
        self.increment_counter(&format!("stream_{}", operation_type));
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        let mut metrics = self.metrics.lock().unwrap();
        let times = self.operation_times.lock().unwrap();
        let counters = self.counters.lock().unwrap();

        // Update cache metrics
        if let Some(cache_lookup_times) = times.get("cache_lookup") {
            metrics.cache_metrics.average_lookup_time_ms = 
                cache_lookup_times.iter().map(|d| d.as_millis() as f64).sum::<f64>() / cache_lookup_times.len() as f64;
        }

        if let Some(cache_store_times) = times.get("cache_store") {
            metrics.cache_metrics.average_store_time_ms = 
                cache_store_times.iter().map(|d| d.as_millis() as f64).sum::<f64>() / cache_store_times.len() as f64;
        }

        let cache_hits = counters.get("cache_lookup_success").unwrap_or(&0);
        let cache_misses = counters.get("cache_lookup_error").unwrap_or(&0);
        let total_cache_ops = cache_hits + cache_misses;
        
        if total_cache_ops > 0 {
            metrics.cache_metrics.hit_ratio = *cache_hits as f64 / total_cache_ops as f64;
        }
        metrics.cache_metrics.total_operations = total_cache_ops;

        // Update template metrics
        if let Some(template_times) = times.get("template_render") {
            metrics.template_metrics.average_render_time_ms = 
                template_times.iter().map(|d| d.as_millis() as f64).sum::<f64>() / template_times.len() as f64;
        }

        let template_success = counters.get("template_render_success").unwrap_or(&0);
        let template_error = counters.get("template_render_error").unwrap_or(&0);
        let total_template_ops = template_success + template_error;
        
        if total_template_ops > 0 {
            metrics.template_metrics.render_success_rate = *template_success as f64 / total_template_ops as f64;
        }
        metrics.template_metrics.total_renders = total_template_ops;

        // Update streaming metrics
        if let Some(first_token_times) = times.get("stream_first_token") {
            metrics.streaming_metrics.average_first_token_time_ms = 
                first_token_times.iter().map(|d| d.as_millis() as f64).sum::<f64>() / first_token_times.len() as f64;
        }

        metrics.streaming_metrics.total_streams_created = counters.get("stream_create").unwrap_or(&0).clone();
        metrics.streaming_metrics.active_streams = counters.get("stream_active").unwrap_or(&0).clone();

        // Update system metrics
        metrics.system_metrics.uptime_seconds = self.start_time.elapsed().as_secs();
        metrics.system_metrics.total_requests = counters.get("total_requests").unwrap_or(&0).clone();

        let total_errors = counters.values().filter(|&v| v > &0).count() as u64;
        if metrics.system_metrics.total_requests > 0 {
            metrics.system_metrics.error_rate = total_errors as f64 / metrics.system_metrics.total_requests as f64;
        }

        metrics.clone()
    }

    pub fn start_monitoring_task(&self) -> tokio::task::JoinHandle<()> {
        let metrics_clone = Arc::clone(&self.metrics);
        let start_time = self.start_time;
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Update every minute
            
            loop {
                interval.tick().await;
                
                // Update system metrics
                let mut metrics = metrics_clone.lock().unwrap();
                metrics.system_metrics.uptime_seconds = start_time.elapsed().as_secs();
                
                // In a real implementation, you would collect actual system metrics here
                // For now, we'll just update the uptime
                
                tracing::debug!("Performance metrics updated: {:?}", *metrics);
            }
        })
    }

    pub fn check_performance_targets(&self) -> PerformanceReport {
        let metrics = self.get_metrics();
        let mut report = PerformanceReport {
            overall_status: PerformanceStatus::Good,
            issues: Vec::new(),
            recommendations: Vec::new(),
        };

        // Check cache performance targets
        if metrics.cache_metrics.average_lookup_time_ms > 10.0 {
            report.issues.push(format!(
                "Cache lookup time ({:.2}ms) exceeds target (10ms)",
                metrics.cache_metrics.average_lookup_time_ms
            ));
            report.overall_status = PerformanceStatus::Warning;
        }

        if metrics.cache_metrics.hit_ratio < 0.8 {
            report.issues.push(format!(
                "Cache hit ratio ({:.1}%) below target (80%)",
                metrics.cache_metrics.hit_ratio * 100.0
            ));
            report.recommendations.push("Consider increasing cache size or adjusting TTL".to_string());
        }

        // Check template performance targets
        if metrics.template_metrics.average_render_time_ms > 50.0 {
            report.issues.push(format!(
                "Template render time ({:.2}ms) exceeds target (50ms)",
                metrics.template_metrics.average_render_time_ms
            ));
            report.overall_status = PerformanceStatus::Warning;
        }

        // Check streaming performance targets
        if metrics.streaming_metrics.average_first_token_time_ms > 200.0 {
            report.issues.push(format!(
                "First token time ({:.2}ms) exceeds target (200ms)",
                metrics.streaming_metrics.average_first_token_time_ms
            ));
            report.overall_status = PerformanceStatus::Critical;
        }

        // Check error rates
        if metrics.system_metrics.error_rate > 0.05 {
            report.issues.push(format!(
                "Error rate ({:.1}%) exceeds target (5%)",
                metrics.system_metrics.error_rate * 100.0
            ));
            report.overall_status = PerformanceStatus::Critical;
        }

        if !report.issues.is_empty() && report.overall_status == PerformanceStatus::Good {
            report.overall_status = PerformanceStatus::Warning;
        }

        report
    }

    pub async fn export_metrics_to_file(&self, path: &str) -> Result<(), std::io::Error> {
        let metrics = self.get_metrics();
        let json = serde_json::to_string_pretty(&metrics)?;
        tokio::fs::write(path, json).await
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub overall_status: PerformanceStatus,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceStatus {
    Good,
    Warning,
    Critical,
}

impl std::fmt::Display for PerformanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PerformanceStatus::Good => write!(f, "✅ Good"),
            PerformanceStatus::Warning => write!(f, "⚠️ Warning"),
            PerformanceStatus::Critical => write!(f, "🚨 Critical"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new();
        
        // Record some operations
        monitor.record_cache_operation("lookup", Duration::from_millis(5), true);
        monitor.record_cache_operation("lookup", Duration::from_millis(15), false);
        monitor.record_template_render(Duration::from_millis(25), true);
        
        let metrics = monitor.get_metrics();
        
        assert!(metrics.cache_metrics.total_operations > 0);
        assert!(metrics.template_metrics.total_renders > 0);
        assert!(metrics.cache_metrics.average_lookup_time_ms > 0.0);
    }

    #[tokio::test]
    async fn test_performance_targets() {
        let monitor = PerformanceMonitor::new();
        
        // Record operations that exceed targets
        monitor.record_cache_operation("lookup", Duration::from_millis(20), true);
        monitor.record_template_render(Duration::from_millis(100), true);
        
        let report = monitor.check_performance_targets();
        
        assert_eq!(report.overall_status, PerformanceStatus::Warning);
        assert!(!report.issues.is_empty());
    }
}