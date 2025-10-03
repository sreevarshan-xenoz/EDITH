use tracing::{info, error, debug};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
    EnvFilter,
};
use tracing_appender::{non_blocking, rolling};
use std::path::Path;
use crate::config::LoggingConfig;

pub fn init_logging(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(filter);

    match config.output.as_str() {
        "file" => {
            let file_path = config.file_path.as_deref().unwrap_or("llm-wrapper.log");
            let file_dir = Path::new(file_path).parent().unwrap_or(Path::new("."));
            let file_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();
            
            let file_appender = rolling::daily(file_dir, file_name);
            let (non_blocking, _guard) = non_blocking(file_appender);
            
            let file_layer = match config.format.as_str() {
                "json" => fmt::layer()
                    .json()
                    .with_writer(non_blocking)
                    .with_span_events(FmtSpan::CLOSE)
                    .boxed(),
                _ => fmt::layer()
                    .with_writer(non_blocking)
                    .with_span_events(FmtSpan::CLOSE)
                    .boxed(),
            };
            
            registry.with(file_layer).init();
        }
        "both" => {
            // Console layer
            let console_layer = match config.format.as_str() {
                "json" => fmt::layer()
                    .json()
                    .with_span_events(FmtSpan::CLOSE)
                    .boxed(),
                _ => fmt::layer()
                    .with_span_events(FmtSpan::CLOSE)
                    .boxed(),
            };
            
            // File layer
            let file_path = config.file_path.as_deref().unwrap_or("llm-wrapper.log");
            let file_dir = Path::new(file_path).parent().unwrap_or(Path::new("."));
            let file_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();
            
            let file_appender = rolling::daily(file_dir, file_name);
            let (non_blocking, _guard) = non_blocking(file_appender);
            
            let file_layer = fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_span_events(FmtSpan::CLOSE)
                .boxed();
            
            registry.with(console_layer).with(file_layer).init();
        }
        _ => {
            // Default to stdout
            let console_layer = match config.format.as_str() {
                "json" => fmt::layer()
                    .json()
                    .with_span_events(FmtSpan::CLOSE)
                    .boxed(),
                _ => fmt::layer()
                    .with_span_events(FmtSpan::CLOSE)
                    .boxed(),
            };
            
            registry.with(console_layer).init();
        }
    }

    info!("Logging initialized with level: {}", config.level);
    Ok(())
}

pub fn log_error(error: &dyn std::error::Error, context: &str) {
    error!(
        error = %error,
        context = context,
        "Error occurred"
    );
    
    // Log error chain
    let mut source = error.source();
    let mut level = 1;
    while let Some(err) = source {
        error!(
            error = %err,
            level = level,
            "Error source"
        );
        source = err.source();
        level += 1;
    }
}

pub fn log_performance_metric(operation: &str, duration_ms: f64, success: bool) {
    info!(
        operation = operation,
        duration_ms = duration_ms,
        success = success,
        "Performance metric"
    );
}

pub fn log_cache_event(event_type: &str, key_hash: u64, hit: bool) {
    debug!(
        event_type = event_type,
        key_hash = key_hash,
        hit = hit,
        "Cache event"
    );
}

pub fn log_template_event(event_type: &str, template_name: &str, success: bool) {
    info!(
        event_type = event_type,
        template_name = template_name,
        success = success,
        "Template event"
    );
}

pub fn log_stream_event(event_type: &str, stream_id: u64, model: &str) {
    info!(
        event_type = event_type,
        stream_id = stream_id,
        model = model,
        "Stream event"
    );
}

pub fn log_backend_event(event_type: &str, backend_name: &str, success: bool, duration_ms: Option<f64>) {
    info!(
        event_type = event_type,
        backend_name = backend_name,
        success = success,
        duration_ms = duration_ms,
        "Backend event"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoggingConfig;

    #[test]
    fn test_logging_init() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            format: "text".to_string(),
            output: "stdout".to_string(),
            file_path: None,
        };

        // This should not panic
        let result = init_logging(&config);
        assert!(result.is_ok());
    }
}