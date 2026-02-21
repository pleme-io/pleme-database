//! Transaction management

use crate::Result;
use async_trait::async_trait;

#[cfg(feature = "postgres")]
use sqlx::{Postgres, Transaction as SqlxTransaction};

/// Transaction wrapper
pub struct Transaction<'a> {
    #[cfg(feature = "postgres")]
    tx: SqlxTransaction<'a, Postgres>,
}

impl<'a> Transaction<'a> {
    /// Commit transaction
    #[cfg(feature = "postgres")]
    pub async fn commit(self) -> Result<()> {
        self.tx
            .commit()
            .await
            .map_err(|e| crate::DatabaseError::TransactionFailed(e.to_string()))
    }

    /// Rollback transaction
    #[cfg(feature = "postgres")]
    pub async fn rollback(self) -> Result<()> {
        self.tx
            .rollback()
            .await
            .map_err(|e| crate::DatabaseError::TransactionFailed(e.to_string()))
    }
}

/// Trait for transactional operations
#[async_trait]
pub trait Transactional {
    /// Begin a transaction
    async fn begin_transaction(&self) -> Result<Transaction>;

    /// Execute within a transaction
    async fn with_transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send,
        T: Send;
}

#[cfg(feature = "postgres")]
#[async_trait]
impl Transactional for crate::DatabasePool {
    async fn begin_transaction(&self) -> Result<Transaction> {
        let tx = self.inner()
            .begin()
            .await
            .map_err(|e| crate::DatabaseError::TransactionFailed(e.to_string()))?;
        Ok(Transaction { tx })
    }

    async fn with_transaction<F, T>(&self, _f: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>> + Send,
        T: Send,
    {
        // Simplified implementation
        unimplemented!("Use begin_transaction() for now")
    }
}
