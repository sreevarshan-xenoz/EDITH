use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use thiserror::Error;
use serde::{Deserialize, Serialize};

pub type StreamId = u64;

#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub max_concurrent_streams: usize,
    pub requests_per_second: f64,
    pub connection_timeout: std::time::Duration,
    pub request_timeout: std::time::Duration,
    pub pool_max_idle_per_host: usize,
}

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Stream not found: {0}")]
    StreamNotFound(StreamId),
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Request timeout")]
    Timeout,
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamToken {
    pub content: String,
    pub is_complete: bool,
    pub metadata: Option<TokenMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub token_count: Option<u32>,
}

pub struct StreamResponse {
    pub id: StreamId,
    pub receiver: mpsc::UnboundedReceiver<StreamToken>,
    pub cancellation_token: CancellationToken,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

pub struct StreamingManager {
    client: reqwest::Client,
    active_streams: HashMap<StreamId, CancellationToken>,
    rate_limiter: RateLimiter,
    next_stream_id: StreamId,
}

pub struct RateLimiter {
    max_concurrent: usize,
    current_count: usize,
    // Token bucket for rate limiting
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: std::time::Instant,
}

impl RateLimiter {
    pub fn new(max_concurrent: usize, requests_per_second: f64) -> Self {
        Self {
            max_concurrent,
            current_count: 0,
            tokens: requests_per_second,
            max_tokens: requests_per_second,
            refill_rate: requests_per_second,
            last_refill: std::time::Instant::now(),
        }
    }

    pub fn can_proceed(&mut self) -> bool {
        self.refill_tokens();
        self.current_count < self.max_concurrent && self.tokens >= 1.0
    }

    pub fn acquire(&mut self) -> bool {
        if self.can_proceed() {
            self.current_count += 1;
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    pub fn release(&mut self) {
        if self.current_count > 0 {
            self.current_count -= 1;
        }
    }

    fn refill_tokens(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        if elapsed > 0.0 {
            let new_tokens = elapsed * self.refill_rate;
            self.tokens = (self.tokens + new_tokens).min(self.max_tokens);
            self.last_refill = now;
        }
    }

    pub fn get_stats(&self) -> RateLimiterStats {
        RateLimiterStats {
            current_concurrent: self.current_count,
            max_concurrent: self.max_concurrent,
            available_tokens: self.tokens,
            max_tokens: self.max_tokens,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub current_concurrent: usize,
    pub max_concurrent: usize,
    pub available_tokens: f64,
    pub max_tokens: f64,
}

impl StreamingManager {
    pub fn new(max_concurrent_streams: usize) -> Self {
        Self::with_config(StreamingConfig {
            max_concurrent_streams,
            requests_per_second: 10.0,
            connection_timeout: std::time::Duration::from_secs(10),
            request_timeout: std::time::Duration::from_secs(30),
            pool_max_idle_per_host: 10,
        })
    }

    pub fn with_config(config: StreamingConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.request_timeout)
            .connect_timeout(config.connection_timeout)
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            active_streams: HashMap::new(),
            rate_limiter: RateLimiter::new(config.max_concurrent_streams, config.requests_per_second),
            next_stream_id: 1,
        }
    }

    pub async fn create_stream(
        &mut self,
        request: ChatRequest,
        base_url: &str,
    ) -> Result<StreamResponse, StreamError> {
        if !self.rate_limiter.acquire() {
            return Err(StreamError::RateLimit);
        }

        let stream_id = self.next_stream_id;
        self.next_stream_id += 1;

        let cancellation_token = CancellationToken::new();
        let (sender, receiver) = mpsc::unbounded_channel();

        // Store the cancellation token
        self.active_streams.insert(stream_id, cancellation_token.clone());

        // Spawn the streaming task
        let client = self.client.clone();
        let url = format!("{}/api/chat", base_url);
        let token = cancellation_token.clone();

        
        tokio::spawn(async move {
            let result = Self::stream_chat(client, url, request, sender, token).await;
            if let Err(e) = result {
                eprintln!("Stream error: {}", e);
            }
        });

        Ok(StreamResponse {
            id: stream_id,
            receiver,
            cancellation_token,
        })
    }

    async fn stream_chat(
        client: reqwest::Client,
        url: String,
        request: ChatRequest,
        sender: mpsc::UnboundedSender<StreamToken>,
        cancellation_token: CancellationToken,
    ) -> Result<(), StreamError> {
        use futures_util::StreamExt;

        let response = Self::send_request_with_retry(&client, &url, &request, 3).await?;

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            // Check for cancellation
            if cancellation_token.is_cancelled() {
                break;
            }

            let chunk = chunk_result?;
            let chunk_str = String::from_utf8_lossy(&chunk);

            // Parse streaming response (assuming JSONL format)
            for line in chunk_str.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(response) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(content) = response.get("message")
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str()) 
                    {
                        let is_complete = response.get("done")
                            .and_then(|d| d.as_bool())
                            .unwrap_or(false);

                        let token = StreamToken {
                            content: content.to_string(),
                            is_complete,
                            metadata: Some(TokenMetadata {
                                timestamp: chrono::Utc::now(),
                                token_count: None,
                            }),
                        };

                        if sender.send(token).is_err() {
                            // Receiver dropped, stop streaming
                            break;
                        }

                        if is_complete {
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn cancel_stream(&mut self, id: StreamId) -> Result<(), StreamError> {
        if let Some(token) = self.active_streams.remove(&id) {
            token.cancel();
            self.rate_limiter.release();
            Ok(())
        } else {
            Err(StreamError::StreamNotFound(id))
        }
    }

    pub fn get_active_streams(&self) -> Vec<StreamId> {
        self.active_streams.keys().copied().collect()
    }

    async fn send_request_with_retry(
        client: &reqwest::Client,
        url: &str,
        request: &ChatRequest,
        max_retries: u32,
    ) -> Result<reqwest::Response, StreamError> {
        let mut attempt = 0;
        let mut delay = std::time::Duration::from_millis(100);

        loop {
            match client.post(url).json(request).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(response);
                    } else if response.status().is_server_error() && attempt < max_retries {
                        // Retry on server errors
                        attempt += 1;
                        tokio::time::sleep(delay).await;
                        delay *= 2; // Exponential backoff
                        continue;
                    } else {
                        return Err(StreamError::Connection(format!(
                            "HTTP error: {}",
                            response.status()
                        )));
                    }
                }
                Err(_e) if attempt < max_retries => {
                    // Retry on connection errors
                    attempt += 1;
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                    continue;
                }
                Err(e) => return Err(StreamError::Http(e)),
            }
        }
    }

    pub fn get_rate_limiter_stats(&mut self) -> RateLimiterStats {
        self.rate_limiter.get_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_streaming_manager_creation() {
        let manager = StreamingManager::new(5);
        assert_eq!(manager.get_active_streams().len(), 0);
        let stats = manager.rate_limiter.get_stats();
        assert_eq!(stats.max_concurrent, 5);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(2, 10.0);
        
        assert!(limiter.acquire());
        assert!(limiter.acquire());
        assert!(!limiter.acquire()); // Should fail, limit reached
        
        limiter.release();
        assert!(limiter.acquire()); // Should work again
    }

    #[tokio::test]
    async fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_stream_token_serialization() {
        let token = StreamToken {
            content: "Hello, world!".to_string(),
            is_complete: false,
            metadata: Some(TokenMetadata {
                timestamp: chrono::Utc::now(),
                token_count: Some(42),
            }),
        };

        let serialized = serde_json::to_string(&token).unwrap();
        let deserialized: StreamToken = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(token.content, deserialized.content);
        assert_eq!(token.is_complete, deserialized.is_complete);
    }

    #[tokio::test]
    async fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
                images: None,
            }],
            stream: true,
            options: None,
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: ChatRequest = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(request.model, deserialized.model);
        assert_eq!(request.messages.len(), deserialized.messages.len());
        assert_eq!(request.stream, deserialized.stream);
    }
}