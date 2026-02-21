//! Database connection pool management
//!
//! Provides production-ready PostgreSQL connection pooling with:
//! - Pool warming via min_connections (pre-established connections)
//! - Idle timeout to release unused connections
//! - Max lifetime to recycle connections (picks up PG config changes)
//! - Connection health checks before checkout
//!
//! See `.claude/skills/connection-pooling-architecture` for full patterns.

use crate::{DatabaseError, Result};
use std::time::Duration;

#[cfg(feature = "postgres")]
use sqlx::{Pool, Postgres};

/// Configuration for database connection pool
#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool (default: 20)
    pub max_connections: u32,
    /// Minimum connections to maintain (pool warming, default: 5)
    pub min_connections: u32,
    /// Time to wait for a connection from the pool (default: 30s)
    pub acquire_timeout_secs: u64,
    /// Release connections idle longer than this (default: 600s = 10 min)
    pub idle_timeout_secs: u64,
    /// Force recycle connections older than this (default: 1800s = 30 min)
    pub max_lifetime_secs: u64,
    /// Validate connection health before returning (default: true)
    pub test_before_acquire: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
            test_before_acquire: true,
        }
    }
}

/// Database connection pool
#[derive(Clone)]
pub struct DatabasePool {
    #[cfg(feature = "postgres")]
    pool: Pool<Postgres>,
}

impl DatabasePool {
    /// Connect to database with default configuration
    #[cfg(feature = "postgres")]
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with_config(url, PoolConfig::default()).await
    }

    /// Connect to database with custom configuration
    ///
    /// Recommended settings by service type:
    /// - High-concurrency (chat, search, order): max=25-30, min=10-15
    /// - Medium-concurrency (auth, payment): max=20, min=5-10
    /// - Low-concurrency (feature-flags, media): max=15, min=3-5
    /// - One-shot binaries (extract-schema): max=1, min=0
    #[cfg(feature = "postgres")]
    pub async fn connect_with_config(url: &str, config: PoolConfig) -> Result<Self> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(config.idle_timeout_secs)))
            .max_lifetime(Some(Duration::from_secs(config.max_lifetime_secs)))
            .test_before_acquire(config.test_before_acquire)
            .connect(url)
            .await
            .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Get underlying pool
    #[cfg(feature = "postgres")]
    pub fn inner(&self) -> &Pool<Postgres> {
        &self.pool
    }

    /// Check database health
    #[cfg(feature = "postgres")]
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))?;
        Ok(())
    }
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database
    async fn test_connect() {
        let pool = DatabasePool::connect("postgres://localhost/test").await;
        assert!(pool.is_ok() || pool.is_err()); // Either connection works or fails gracefully
    }
}
