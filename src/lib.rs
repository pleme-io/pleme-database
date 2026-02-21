//! # pleme-database
//!
//! Database utilities library for Pleme platform services.
//!
//! ## Features
//!
//! - **Connection Pooling** - Managed PostgreSQL connection pools
//! - **Transactions** - Transactional operations with rollback
//! - **Repository Pattern** - Clean data access layer abstraction
//! - **Cache Integration** - Redis caching with automatic invalidation
//!
//! ## Usage
//!
//! ```rust,no_run
//! use pleme_database::{DatabasePool, Repository};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let pool = DatabasePool::connect("postgres://localhost/mydb").await?;
//!
//!     // Use repository pattern
//!     let users = UserRepository::new(pool);
//!     let user = users.find_by_id("123").await?;
//!
//!     Ok(())
//! }
//! ```

pub mod pool;
pub mod transaction;
pub mod repository;

#[cfg(feature = "cache")]
pub mod cache;

pub use pool::{DatabasePool, PoolConfig};
pub use transaction::{Transaction, Transactional};
pub use repository::{
    Repository, SoftDelete, ProductScoped, PaginatedRepository,
    PaginationParams, PaginatedResponse, BaseRepository,
};

#[cfg(feature = "cache")]
pub use cache::{CacheManager, CacheAside};

use thiserror::Error;

/// Database errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[cfg(feature = "cache")]
    #[error("Cache error: {0}")]
    CacheError(String),
}

/// Result type for database operations
pub type Result<T> = std::result::Result<T, DatabaseError>;
