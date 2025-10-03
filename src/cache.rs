use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use thiserror::Error;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::sync::mpsc;
use crate::streaming::{StreamToken, StreamResponse, StreamId};

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Cache full")]
    CacheFull,
    #[error("Memory pressure detected")]
    MemoryPressure,
    #[error("Persistence error: {0}")]
    Persistence(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_entries: usize,
    pub memory_usage_bytes: usize,
    pub evictions: u64,
    pub disk_writes: u64,
    pub disk_reads: u64,
}

impl CacheStats {
    pub fn hit_ratio(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    pub fn eviction_ratio(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.evictions as f64 / self.total_entries as f64
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    pub prompt_hash: u64,
    pub model: String,
    pub parameters: ParameterHash,
}

impl CacheKey {
    pub fn new(prompt: &str, model: &str, parameters: &HashMap<String, serde_json::Value>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        let prompt_hash = u64::from_le_bytes(hasher.finalize()[..8].try_into().unwrap());

        Self {
            prompt_hash,
            model: model.to_string(),
            parameters: ParameterHash::new(parameters),
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ParameterHash(u64);

impl ParameterHash {
    pub fn new(parameters: &HashMap<String, serde_json::Value>) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        
        // Sort parameters for consistent hashing
        let mut sorted_params: Vec<_> = parameters.iter().collect();
        sorted_params.sort_by_key(|(k, _)| *k);
        
        for (key, value) in sorted_params {
            key.hash(&mut hasher);
            // Simple hash for JSON values
            value.to_string().hash(&mut hasher);
        }
        
        Self(hasher.finish())
    }
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub response: String,
    pub created_at: Instant,
    pub access_count: u32,
    pub metadata: ResponseMetadata,
    pub is_streaming: bool,
    pub stream_tokens: Option<Vec<StreamToken>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub model: String,
    pub tokens_used: Option<u32>,
    pub response_time: Duration,
    pub backend_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_memory_entries: usize,
    #[serde(with = "humantime_serde")]
    pub ttl: Duration,
    pub enable_persistence: bool,
    pub cache_streaming: bool,
    pub cache_dir: Option<PathBuf>,
    pub max_memory_bytes: Option<usize>,
    pub memory_pressure_threshold: f64, // 0.0 to 1.0
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_entries: 1000,
            ttl: Duration::from_secs(3600), // 1 hour
            enable_persistence: false,
            cache_streaming: true,
            cache_dir: Some(PathBuf::from(".cache")),
            max_memory_bytes: Some(100 * 1024 * 1024), // 100MB
            memory_pressure_threshold: 0.8, // 80%
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistentCacheEntry {
    response: String,
    created_at: std::time::SystemTime,
    access_count: u32,
    metadata: ResponseMetadata,
    is_streaming: bool,
    stream_tokens: Option<Vec<StreamToken>>,
}

impl From<&CacheEntry> for PersistentCacheEntry {
    fn from(entry: &CacheEntry) -> Self {
        Self {
            response: entry.response.clone(),
            created_at: std::time::SystemTime::now() - entry.created_at.elapsed(),
            access_count: entry.access_count,
            metadata: entry.metadata.clone(),
            is_streaming: entry.is_streaming,
            stream_tokens: entry.stream_tokens.clone(),
        }
    }
}

impl From<PersistentCacheEntry> for CacheEntry {
    fn from(entry: PersistentCacheEntry) -> Self {
        let created_at = entry.created_at
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| Instant::now() - d)
            .unwrap_or_else(|_| Instant::now());

        Self {
            response: entry.response,
            created_at,
            access_count: entry.access_count,
            metadata: entry.metadata,
            is_streaming: entry.is_streaming,
            stream_tokens: entry.stream_tokens,
        }
    }
}

pub struct CacheManager {
    memory_cache: LruCache<CacheKey, CacheEntry>,
    config: CacheConfig,
    stats: CacheStats,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        let capacity = NonZeroUsize::new(config.max_memory_entries)
            .unwrap_or(NonZeroUsize::new(1000).unwrap());
        
        Self {
            memory_cache: LruCache::new(capacity),
            config,
            stats: CacheStats {
                hits: 0,
                misses: 0,
                total_entries: 0,
                memory_usage_bytes: 0,
                evictions: 0,
                disk_writes: 0,
                disk_reads: 0,
            },
        }
    }

    pub async fn new_with_persistence(config: CacheConfig) -> Result<Self, CacheError> {
        let mut cache_manager = Self::new(config);
        
        if cache_manager.config.enable_persistence {
            cache_manager.load_from_disk().await?;
        }
        
        Ok(cache_manager)
    }

    pub async fn get(&mut self, key: &CacheKey) -> Option<String> {
        // First check memory cache
        if let Some(entry) = self.memory_cache.get_mut(key) {
            // Check TTL
            if entry.created_at.elapsed() > self.config.ttl {
                self.memory_cache.pop(key);
                self.stats.misses += 1;
                return None;
            }

            // Update access count
            entry.access_count += 1;
            self.stats.hits += 1;
            return Some(entry.response.clone());
        }

        // If not in memory and persistence is enabled, try disk
        if self.config.enable_persistence {
            if let Ok(Some(entry)) = self.load_from_disk_by_key(key).await {
                // Check TTL for disk entry
                if entry.created_at.elapsed() <= self.config.ttl {
                    let response = entry.response.clone();
                    
                    // Put back in memory cache
                    let mut updated_entry = entry;
                    updated_entry.access_count += 1;
                    self.memory_cache.put(key.clone(), updated_entry);
                    
                    self.stats.hits += 1;
                    self.stats.disk_reads += 1;
                    return Some(response);
                }
            }
        }

        self.stats.misses += 1;
        None
    }

    pub async fn put(
        &mut self,
        key: CacheKey,
        value: String,
        metadata: ResponseMetadata,
    ) -> Result<(), CacheError> {
        let entry = CacheEntry {
            response: value.clone(),
            created_at: Instant::now(),
            access_count: 1,
            metadata: metadata.clone(),
            is_streaming: false,
            stream_tokens: None,
        };

        // Check memory pressure before adding
        self.handle_memory_pressure().await?;

        // Store in memory cache
        if let Some(evicted) = self.memory_cache.push(key.clone(), entry.clone()) {
            self.stats.evictions += 1;
            
            // If persistence is enabled, save evicted entry to disk
            if self.config.enable_persistence {
                self.save_to_disk(&evicted.0, &evicted.1).await?;
            }
        }

        // Also save to disk if persistence is enabled
        if self.config.enable_persistence {
            self.save_to_disk(&key, &entry).await?;
        }

        self.update_stats();
        Ok(())
    }

    pub fn invalidate_model(&mut self, model: &str) {
        let keys_to_remove: Vec<_> = self.memory_cache
            .iter()
            .filter(|(key, _)| key.model == model)
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            self.memory_cache.pop(&key);
        }

        self.update_stats();
    }

    pub fn invalidate_by_parameters(&mut self, model: &str, parameters: &HashMap<String, serde_json::Value>) {
        let target_param_hash = ParameterHash::new(parameters);
        
        let keys_to_remove: Vec<_> = self.memory_cache
            .iter()
            .filter(|(key, _)| key.model == model && key.parameters == target_param_hash)
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            self.memory_cache.pop(&key);
        }

        self.update_stats();
    }

    pub fn invalidate_expired(&mut self) {
        let now = Instant::now();
        let keys_to_remove: Vec<_> = self.memory_cache
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.created_at) > self.config.ttl)
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            self.memory_cache.pop(&key);
        }

        self.update_stats();
    }

    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }

    pub async fn persist_to_disk(&mut self) -> Result<(), CacheError> {
        if !self.config.enable_persistence {
            return Ok(());
        }

        let cache_dir = self.get_cache_dir()?;
        fs::create_dir_all(&cache_dir).await
            .map_err(|e| CacheError::Persistence(format!("Failed to create cache directory: {}", e)))?;

        // Save all memory cache entries to disk
        let entries: Vec<_> = self.memory_cache.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
            
        for (key, entry) in entries {
            self.save_to_disk(&key, &entry).await?;
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.memory_cache.clear();
        self.stats.total_entries = 0;
        self.stats.memory_usage_bytes = 0;
    }

    async fn handle_memory_pressure(&mut self) -> Result<(), CacheError> {
        if let Some(max_bytes) = self.config.max_memory_bytes {
            let current_usage = self.estimate_memory_usage();
            let threshold = (max_bytes as f64 * self.config.memory_pressure_threshold) as usize;
            
            if current_usage > threshold {
                // Reduce cache size by 25%
                let target_size = (self.memory_cache.len() as f64 * 0.75) as usize;
                
                while self.memory_cache.len() > target_size {
                    if let Some((key, entry)) = self.memory_cache.pop_lru() {
                        self.stats.evictions += 1;
                        
                        // Save to disk if persistence is enabled
                        if self.config.enable_persistence {
                            self.save_to_disk(&key, &entry).await?;
                        }
                    } else {
                        break;
                    }
                }
                
                self.update_stats();
            }
        }
        
        Ok(())
    }

    fn estimate_memory_usage(&self) -> usize {
        // Rough estimation: each entry is approximately the size of the response plus overhead
        self.memory_cache.iter()
            .map(|(key, entry)| {
                key.model.len() + 
                entry.response.len() + 
                entry.metadata.model.len() + 
                entry.metadata.backend_type.len() + 
                200 // overhead estimate
            })
            .sum()
    }

    fn update_stats(&mut self) {
        self.stats.total_entries = self.memory_cache.len();
        self.stats.memory_usage_bytes = self.estimate_memory_usage();
    }

    async fn save_to_disk(&mut self, key: &CacheKey, entry: &CacheEntry) -> Result<(), CacheError> {
        let cache_dir = self.get_cache_dir()?;
        fs::create_dir_all(&cache_dir).await
            .map_err(|e| CacheError::Persistence(format!("Failed to create cache directory: {}", e)))?;
            
        let file_path = cache_dir.join(format!("{:x}.json", key.prompt_hash));
        
        let persistent_entry = PersistentCacheEntry::from(entry);
        let serialized = serde_json::to_string(&persistent_entry)?;
        
        fs::write(&file_path, serialized).await
            .map_err(|e| CacheError::Persistence(format!("Failed to write cache file: {}", e)))?;
        
        self.stats.disk_writes += 1;
        Ok(())
    }

    async fn load_from_disk_by_key(&self, key: &CacheKey) -> Result<Option<CacheEntry>, CacheError> {
        let cache_dir = self.get_cache_dir()?;
        let file_path = cache_dir.join(format!("{:x}.json", key.prompt_hash));
        
        if !file_path.exists() {
            return Ok(None);
        }
        
        let content = fs::read_to_string(&file_path).await
            .map_err(|e| CacheError::Persistence(format!("Failed to read cache file: {}", e)))?;
        
        let persistent_entry: PersistentCacheEntry = serde_json::from_str(&content)?;
        Ok(Some(persistent_entry.into()))
    }

    async fn load_from_disk(&mut self) -> Result<(), CacheError> {
        let cache_dir = self.get_cache_dir()?;
        
        if !cache_dir.exists() {
            return Ok(());
        }
        
        let mut entries = fs::read_dir(&cache_dir).await
            .map_err(|e| CacheError::Persistence(format!("Failed to read cache directory: {}", e)))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| CacheError::Persistence(format!("Failed to read directory entry: {}", e)))? {
            
            if let Some(extension) = entry.path().extension() {
                if extension == "json" {
                    if let Ok(content) = fs::read_to_string(entry.path()).await {
                        if let Ok(persistent_entry) = serde_json::from_str::<PersistentCacheEntry>(&content) {
                            let cache_entry: CacheEntry = persistent_entry.into();
                            
                            // Check TTL before loading
                            if cache_entry.created_at.elapsed() <= self.config.ttl {
                                // Create a dummy key for loading - in practice, we'd need to store the key
                                // For now, we'll skip loading from disk on startup to avoid this complexity
                                // This would be improved in a production implementation
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    fn get_cache_dir(&self) -> Result<PathBuf, CacheError> {
        self.config.cache_dir.clone()
            .ok_or_else(|| CacheError::Persistence("Cache directory not configured".to_string()))
    }

    pub fn reduce_cache_size(&mut self, target_ratio: f64) {
        let target_size = (self.memory_cache.len() as f64 * target_ratio) as usize;
        
        while self.memory_cache.len() > target_size {
            if self.memory_cache.pop_lru().is_some() {
                self.stats.evictions += 1;
            } else {
                break;
            }
        }
        
        self.update_stats();
    }

    pub fn get_detailed_stats(&self) -> DetailedCacheStats {
        DetailedCacheStats {
            basic_stats: self.stats.clone(),
            memory_pressure_ratio: self.get_memory_pressure_ratio(),
            average_entry_size: self.get_average_entry_size(),
            cache_efficiency: self.calculate_cache_efficiency(),
        }
    }

    fn get_memory_pressure_ratio(&self) -> f64 {
        if let Some(max_bytes) = self.config.max_memory_bytes {
            self.stats.memory_usage_bytes as f64 / max_bytes as f64
        } else {
            0.0
        }
    }

    fn get_average_entry_size(&self) -> usize {
        if self.memory_cache.is_empty() {
            0
        } else {
            self.stats.memory_usage_bytes / self.memory_cache.len()
        }
    }

    fn calculate_cache_efficiency(&self) -> f64 {
        let total_requests = self.stats.hits + self.stats.misses;
        if total_requests == 0 {
            0.0
        } else {
            // Efficiency considers both hit ratio and eviction ratio
            let hit_ratio = self.stats.hit_ratio();
            let eviction_penalty = if self.stats.total_entries > 0 {
                self.stats.evictions as f64 / self.stats.total_entries as f64
            } else {
                0.0
            };
            
            hit_ratio * (1.0 - eviction_penalty * 0.1) // Small penalty for evictions
        }
    }

    pub async fn put_streaming(
        &mut self,
        key: CacheKey,
        tokens: Vec<StreamToken>,
        metadata: ResponseMetadata,
    ) -> Result<(), CacheError> {
        if !self.config.cache_streaming {
            return Ok(());
        }

        // Combine all tokens into a single response
        let response = tokens.iter()
            .map(|token| token.content.as_str())
            .collect::<Vec<_>>()
            .join("");

        let entry = CacheEntry {
            response,
            created_at: Instant::now(),
            access_count: 1,
            metadata: metadata.clone(),
            is_streaming: true,
            stream_tokens: Some(tokens),
        };

        // Check memory pressure before adding
        self.handle_memory_pressure().await?;

        // Store in memory cache
        if let Some(evicted) = self.memory_cache.push(key.clone(), entry.clone()) {
            self.stats.evictions += 1;
            
            // If persistence is enabled, save evicted entry to disk
            if self.config.enable_persistence {
                self.save_to_disk(&evicted.0, &evicted.1).await?;
            }
        }

        // Also save to disk if persistence is enabled
        if self.config.enable_persistence {
            self.save_to_disk(&key, &entry).await?;
        }

        self.update_stats();
        Ok(())
    }

    pub async fn get_streaming(&mut self, key: &CacheKey) -> Option<Vec<StreamToken>> {
        if let Some(entry) = self.memory_cache.get_mut(key) {
            // Check TTL
            if entry.created_at.elapsed() > self.config.ttl {
                self.memory_cache.pop(key);
                self.stats.misses += 1;
                return None;
            }

            // Update access count
            entry.access_count += 1;
            self.stats.hits += 1;
            
            if entry.is_streaming {
                return entry.stream_tokens.clone();
            }
        }

        // If not in memory and persistence is enabled, try disk
        if self.config.enable_persistence {
            if let Ok(Some(entry)) = self.load_from_disk_by_key(key).await {
                // Check TTL for disk entry
                if entry.created_at.elapsed() <= self.config.ttl && entry.is_streaming {
                    let tokens = entry.stream_tokens.clone();
                    
                    // Put back in memory cache
                    let mut updated_entry = entry;
                    updated_entry.access_count += 1;
                    self.memory_cache.put(key.clone(), updated_entry);
                    
                    self.stats.hits += 1;
                    self.stats.disk_reads += 1;
                    return tokens;
                }
            }
        }

        self.stats.misses += 1;
        None
    }

    pub async fn create_cached_stream(
        &mut self,
        key: &CacheKey,
        stream_id: StreamId,
    ) -> Option<StreamResponse> {
        if let Some(tokens) = self.get_streaming(key).await {
            let (sender, receiver) = mpsc::unbounded_channel();
            let cancellation_token = tokio_util::sync::CancellationToken::new();
            
            // Spawn a task to replay the cached tokens
            let token_clone = cancellation_token.clone();
            tokio::spawn(async move {
                for token in tokens {
                    if token_clone.is_cancelled() {
                        break;
                    }
                    
                    if sender.send(token.clone()).is_err() {
                        break;
                    }
                    
                    // Add small delay to simulate streaming
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            });

            Some(StreamResponse {
                id: stream_id,
                receiver,
                cancellation_token,
            })
        } else {
            None
        }
    }

    pub async fn warm_cache(&mut self, keys: Vec<CacheKey>) -> Result<(), CacheError> {
        if !self.config.enable_persistence {
            return Ok(());
        }

        for key in keys {
            if !self.memory_cache.contains(&key) {
                if let Ok(Some(entry)) = self.load_from_disk_by_key(&key).await {
                    // Check TTL before warming
                    if entry.created_at.elapsed() <= self.config.ttl {
                        self.memory_cache.put(key, entry);
                        self.stats.disk_reads += 1;
                    }
                }
            }
        }

        self.update_stats();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedCacheStats {
    pub basic_stats: CacheStats,
    pub memory_pressure_ratio: f64,
    pub average_entry_size: usize,
    pub cache_efficiency: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::time::{sleep, Duration};

    fn create_test_config() -> CacheConfig {
        CacheConfig {
            max_memory_entries: 3,
            ttl: Duration::from_secs(1),
            enable_persistence: false,
            cache_streaming: true,
            cache_dir: Some(PathBuf::from("test_cache")),
            max_memory_bytes: Some(1024),
            memory_pressure_threshold: 0.8,
        }
    }

    fn create_test_metadata() -> ResponseMetadata {
        ResponseMetadata {
            model: "test-model".to_string(),
            tokens_used: Some(100),
            response_time: Duration::from_millis(500),
            backend_type: "test".to_string(),
        }
    }

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let mut cache = CacheManager::new(create_test_config());
        let key = CacheKey::new("test prompt", "test-model", &HashMap::new());
        let response = "test response".to_string();
        let metadata = create_test_metadata();

        cache.put(key.clone(), response.clone(), metadata).await.unwrap();
        
        let retrieved = cache.get(&key).await;
        assert_eq!(retrieved, Some(response));
        assert_eq!(cache.stats.hits, 1);
        assert_eq!(cache.stats.misses, 0);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let mut cache = CacheManager::new(create_test_config());
        let key = CacheKey::new("nonexistent prompt", "test-model", &HashMap::new());
        
        let retrieved = cache.get(&key).await;
        assert_eq!(retrieved, None);
        assert_eq!(cache.stats.hits, 0);
        assert_eq!(cache.stats.misses, 1);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let mut cache = CacheManager::new(create_test_config());
        let metadata = create_test_metadata();

        // Fill cache to capacity
        for i in 0..3 {
            let key = CacheKey::new(&format!("prompt {}", i), "test-model", &HashMap::new());
            cache.put(key, format!("response {}", i), metadata.clone()).await.unwrap();
        }

        // Add one more item to trigger eviction
        let key4 = CacheKey::new("prompt 3", "test-model", &HashMap::new());
        cache.put(key4.clone(), "response 3".to_string(), metadata).await.unwrap();

        // First item should be evicted
        let key0 = CacheKey::new("prompt 0", "test-model", &HashMap::new());
        let retrieved = cache.get(&key0).await;
        assert_eq!(retrieved, None);

        // Last item should still be there
        let retrieved = cache.get(&key4).await;
        assert_eq!(retrieved, Some("response 3".to_string()));
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let mut cache = CacheManager::new(create_test_config());
        let key = CacheKey::new("test prompt", "test-model", &HashMap::new());
        let response = "test response".to_string();
        let metadata = create_test_metadata();

        cache.put(key.clone(), response.clone(), metadata).await.unwrap();
        
        // Should be available immediately
        let retrieved = cache.get(&key).await;
        assert_eq!(retrieved, Some(response));

        // Wait for TTL to expire
        sleep(Duration::from_secs(2)).await;

        // Should be expired now
        let retrieved = cache.get(&key).await;
        assert_eq!(retrieved, None);
        assert_eq!(cache.stats.misses, 1);
    }

    #[tokio::test]
    async fn test_model_invalidation() {
        let mut cache = CacheManager::new(create_test_config());
        let metadata = create_test_metadata();

        // Add entries for different models
        let key1 = CacheKey::new("prompt 1", "model-a", &HashMap::new());
        let key2 = CacheKey::new("prompt 2", "model-b", &HashMap::new());
        
        cache.put(key1.clone(), "response 1".to_string(), metadata.clone()).await.unwrap();
        cache.put(key2.clone(), "response 2".to_string(), metadata).await.unwrap();

        // Invalidate model-a
        cache.invalidate_model("model-a");

        // model-a entry should be gone
        let retrieved = cache.get(&key1).await;
        assert_eq!(retrieved, None);

        // model-b entry should still be there
        let retrieved = cache.get(&key2).await;
        assert_eq!(retrieved, Some("response 2".to_string()));
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let mut cache = CacheManager::new(create_test_config());
        let key = CacheKey::new("test prompt", "test-model", &HashMap::new());
        let metadata = create_test_metadata();

        // Initial stats
        assert_eq!(cache.stats.hit_ratio(), 0.0);

        // Add entry and access it
        cache.put(key.clone(), "response".to_string(), metadata).await.unwrap();
        cache.get(&key).await;
        cache.get(&key).await;

        // Miss on non-existent key
        let key2 = CacheKey::new("other prompt", "test-model", &HashMap::new());
        cache.get(&key2).await;

        assert_eq!(cache.stats.hits, 2);
        assert_eq!(cache.stats.misses, 1);
        assert_eq!(cache.stats.hit_ratio(), 2.0 / 3.0);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let params1 = HashMap::from([
            ("temperature".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.7).unwrap())),
            ("max_tokens".to_string(), serde_json::Value::Number(serde_json::Number::from(100))),
        ]);

        let params2 = HashMap::from([
            ("max_tokens".to_string(), serde_json::Value::Number(serde_json::Number::from(100))),
            ("temperature".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(0.7).unwrap())),
        ]);

        let key1 = CacheKey::new("test prompt", "model", &params1);
        let key2 = CacheKey::new("test prompt", "model", &params2);

        // Keys should be equal regardless of parameter order
        assert_eq!(key1, key2);

        // Different prompts should generate different keys
        let key3 = CacheKey::new("different prompt", "model", &params1);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_memory_pressure_handling() {
        let config = CacheConfig {
            max_memory_entries: 10,
            ttl: Duration::from_secs(3600),
            enable_persistence: false,
            cache_streaming: true,
            cache_dir: Some(PathBuf::from("test_cache")),
            max_memory_bytes: Some(100), // Very small limit to trigger pressure
            memory_pressure_threshold: 0.5,
        };

        let mut cache = CacheManager::new(config);
        let metadata = create_test_metadata();

        // Add several large entries
        for i in 0..5 {
            let key = CacheKey::new(&format!("prompt {}", i), "test-model", &HashMap::new());
            let large_response = "x".repeat(50); // Large response to trigger memory pressure
            cache.put(key, large_response, metadata.clone()).await.unwrap();
        }

        // Cache should have been reduced due to memory pressure
        assert!(cache.memory_cache.len() < 5);
        assert!(cache.stats.evictions > 0);
    }

    #[tokio::test]
    async fn test_streaming_cache() {
        let mut cache = CacheManager::new(create_test_config());
        let key = CacheKey::new("test prompt", "test-model", &HashMap::new());
        let metadata = create_test_metadata();

        let tokens = vec![
            StreamToken {
                content: "Hello".to_string(),
                is_complete: false,
                metadata: None,
            },
            StreamToken {
                content: " world!".to_string(),
                is_complete: true,
                metadata: None,
            },
        ];

        cache.put_streaming(key.clone(), tokens.clone(), metadata).await.unwrap();
        
        let retrieved_tokens = cache.get_streaming(&key).await;
        assert!(retrieved_tokens.is_some());
        let retrieved = retrieved_tokens.unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].content, "Hello");
        assert_eq!(retrieved[1].content, " world!");
        assert_eq!(cache.stats.hits, 1);
    }

    #[tokio::test]
    async fn test_cached_stream_creation() {
        let mut cache = CacheManager::new(create_test_config());
        let key = CacheKey::new("test prompt", "test-model", &HashMap::new());
        let metadata = create_test_metadata();

        let tokens = vec![
            StreamToken {
                content: "Test".to_string(),
                is_complete: false,
                metadata: None,
            },
            StreamToken {
                content: " response".to_string(),
                is_complete: true,
                metadata: None,
            },
        ];

        cache.put_streaming(key.clone(), tokens, metadata).await.unwrap();
        
        let stream = cache.create_cached_stream(&key, 123).await;
        assert!(stream.is_some());
        
        let mut stream = stream.unwrap();
        assert_eq!(stream.id, 123);
        
        // Should be able to receive tokens from the cached stream
        let first_token = timeout(Duration::from_secs(1), stream.receiver.recv()).await;
        assert!(first_token.is_ok());
        assert!(first_token.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_cache_warming() {
        let config = CacheConfig {
            enable_persistence: false, // Disable persistence for this test
            ..create_test_config()
        };
        let mut cache = CacheManager::new(config);
        
        let key1 = CacheKey::new("prompt 1", "test-model", &HashMap::new());
        let key2 = CacheKey::new("prompt 2", "test-model", &HashMap::new());
        let keys = vec![key1.clone(), key2.clone()];
        
        // Warming should not fail even if keys don't exist
        let result = cache.warm_cache(keys).await;
        assert!(result.is_ok());
    }
}