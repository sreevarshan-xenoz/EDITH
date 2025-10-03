use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use thiserror::Error;
use sha2::{Digest, Sha256};

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Cache full")]
    CacheFull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_entries: usize,
    pub memory_usage_bytes: usize,
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
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_memory_entries: 1000,
            ttl: Duration::from_secs(3600), // 1 hour
            enable_persistence: false,
            cache_streaming: true,
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
            },
        }
    }

    pub async fn get(&mut self, key: &CacheKey) -> Option<String> {
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
            Some(entry.response.clone())
        } else {
            self.stats.misses += 1;
            None
        }
    }

    pub async fn put(
        &mut self,
        key: CacheKey,
        value: String,
        metadata: ResponseMetadata,
    ) -> Result<(), CacheError> {
        let entry = CacheEntry {
            response: value,
            created_at: Instant::now(),
            access_count: 1,
            metadata,
        };

        self.memory_cache.put(key, entry);
        self.stats.total_entries = self.memory_cache.len();
        
        // Rough memory usage estimation
        self.stats.memory_usage_bytes = self.memory_cache.len() * 1024; // Rough estimate
        
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

        self.stats.total_entries = self.memory_cache.len();
    }

    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }

    pub async fn persist_to_disk(&self) -> Result<(), CacheError> {
        if !self.config.enable_persistence {
            return Ok(());
        }

        // TODO: Implement disk persistence
        // For now, this is a placeholder
        Ok(())
    }

    pub fn clear(&mut self) {
        self.memory_cache.clear();
        self.stats.total_entries = 0;
        self.stats.memory_usage_bytes = 0;
    }
}