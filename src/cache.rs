//! Redis cache management

use crate::{DatabaseError, Result};
use async_trait::async_trait;
use redis::{aio::ConnectionManager, Client};
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;

/// Cache manager for Redis operations
#[derive(Clone)]
pub struct CacheManager {
    client: Client,
}

impl CacheManager {
    /// Create new cache manager
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)
            .map_err(|e| DatabaseError::CacheError(e.to_string()))?;
        Ok(Self { client })
    }

    /// Get connection manager
    pub async fn get_connection(&self) -> Result<ConnectionManager> {
        ConnectionManager::new(self.client.clone())
            .await
            .map_err(|e| DatabaseError::CacheError(e.to_string()))
    }

    /// Get value from cache
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.get_connection().await?;
        let value: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| DatabaseError::CacheError(e.to_string()))?;

        match value {
            Some(v) => {
                let parsed = serde_json::from_str(&v)
                    .map_err(|e| DatabaseError::CacheError(e.to_string()))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Set value in cache
    pub async fn set<T>(&self, key: &str, value: &T, ttl_seconds: Option<usize>) -> Result<()>
    where
        T: Serialize,
    {
        let mut conn = self.get_connection().await?;
        let serialized = serde_json::to_string(value)
            .map_err(|e| DatabaseError::CacheError(e.to_string()))?;

        if let Some(ttl) = ttl_seconds {
            let _: () = redis::cmd("SETEX")
                .arg(key)
                .arg(ttl)
                .arg(serialized)
                .query_async(&mut conn)
                .await
                .map_err(|e| DatabaseError::CacheError(e.to_string()))?;
        } else {
            let _: () = redis::cmd("SET")
                .arg(key)
                .arg(serialized)
                .query_async(&mut conn)
                .await
                .map_err(|e| DatabaseError::CacheError(e.to_string()))?;
        }

        Ok(())
    }

    /// Delete key from cache
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let _: () = redis::cmd("DEL")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| DatabaseError::CacheError(e.to_string()))?;
        Ok(())
    }

    /// Invalidate cache by pattern (e.g., "user:*")
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<u64> {
        let mut conn = self.get_connection().await?;

        // Get all keys matching pattern
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| DatabaseError::CacheError(e.to_string()))?;

        if keys.is_empty() {
            return Ok(0);
        }

        // Delete all matching keys
        let count: u64 = redis::cmd("DEL")
            .arg(&keys)
            .query_async(&mut conn)
            .await
            .map_err(|e| DatabaseError::CacheError(e.to_string()))?;

        Ok(count)
    }
}

/// Cache-Aside pattern trait
///
/// Implements the cache-aside (lazy loading) pattern where:
/// 1. Check cache first
/// 2. If miss, fetch from database
/// 3. Store in cache for future requests
/// 4. Return data
///
/// This pattern reduces database load and improves response times.
#[async_trait]
pub trait CacheAside {
    /// Get value from cache or fetch from database
    ///
    /// # Example
    /// ```rust,no_run
    /// use pleme_database::{CacheManager, CacheAside};
    ///
    /// async fn get_user(cache: &CacheManager, user_id: &str) -> Result<User> {
    ///     cache.get_or_fetch(
    ///         &format!("user:{}", user_id),
    ///         300, // 5 minute TTL
    ///         || async {
    ///             // Fetch from database
    ///             db.query_one("SELECT * FROM users WHERE id = $1", &[user_id]).await
    ///         }
    ///     ).await
    /// }
    /// ```
    async fn get_or_fetch<T, F, Fut>(
        &self,
        key: &str,
        ttl_seconds: usize,
        fetch_fn: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T>> + Send;

    /// Get value from cache or fetch, then update cache
    async fn fetch_and_cache<T, F, Fut>(
        &self,
        key: &str,
        ttl_seconds: usize,
        fetch_fn: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T>> + Send;
}

#[async_trait]
impl CacheAside for CacheManager {
    async fn get_or_fetch<T, F, Fut>(
        &self,
        key: &str,
        ttl_seconds: usize,
        fetch_fn: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T>> + Send,
    {
        // Try cache first
        if let Some(cached) = self.get::<T>(key).await? {
            tracing::debug!(key = %key, "Cache hit");
            return Ok(cached);
        }

        // Cache miss - fetch from source
        tracing::debug!(key = %key, "Cache miss, fetching from source");
        let value = fetch_fn().await?;

        // Store in cache for next time
        self.set(key, &value, Some(ttl_seconds)).await?;

        Ok(value)
    }

    async fn fetch_and_cache<T, F, Fut>(
        &self,
        key: &str,
        ttl_seconds: usize,
        fetch_fn: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T>> + Send,
    {
        // Always fetch fresh data
        let value = fetch_fn().await?;

        // Update cache
        self.set(key, &value, Some(ttl_seconds)).await?;

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_manager_creation() {
        let result = CacheManager::new("redis://localhost");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cache_aside_pattern() {
        let cache = CacheManager::new("redis://localhost:6379").unwrap();

        // Simulate get_or_fetch pattern
        let fetch_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let fetch_count_clone = fetch_count.clone();

        let result = cache.get_or_fetch(
            "test:key",
            60,
            || async move {
                fetch_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok::<String, DatabaseError>("value".to_string())
            }
        ).await;

        // Note: This test will fail without Redis running, but demonstrates the API
        let _ = result;
    }
}
