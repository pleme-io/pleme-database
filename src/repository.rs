//! Repository pattern for data access

use crate::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

/// Repository trait for CRUD operations
#[async_trait]
pub trait Repository<T>: Send + Sync
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    /// Find entity by ID
    async fn find_by_id(&self, id: &str) -> Result<Option<T>>;

    /// Find all entities
    async fn find_all(&self) -> Result<Vec<T>>;

    /// Create new entity
    async fn create(&self, entity: &T) -> Result<T>;

    /// Update existing entity
    async fn update(&self, id: &str, entity: &T) -> Result<T>;

    /// Delete entity
    async fn delete(&self, id: &str) -> Result<()>;
}

/// Soft delete trait for logical deletion
///
/// Implements the soft delete pattern where records are marked as deleted
/// rather than physically removed from the database. This allows for:
/// - Data recovery
/// - Audit trails
/// - Referential integrity maintenance
#[async_trait]
pub trait SoftDelete: Send + Sync {
    /// Soft delete entity by marking it as deleted
    async fn soft_delete(&self, id: &str) -> Result<()>;

    /// Restore soft-deleted entity
    async fn restore(&self, id: &str) -> Result<()>;

    /// Check if entity is soft-deleted
    async fn is_deleted(&self, id: &str) -> Result<bool>;

    /// Find all entities including soft-deleted ones
    async fn find_all_with_deleted(&self) -> Result<Vec<serde_json::Value>>;

    /// Permanently delete soft-deleted entities (hard delete)
    async fn purge_deleted(&self, older_than_days: u32) -> Result<u64>;
}

/// Product-scoped trait for multi-tenant data isolation
///
/// Implements product-level data isolation where all queries are automatically
/// scoped to a specific product ID. This ensures data isolation between
/// products (Lilitu, NovaSkyn, etc.) in the Pleme platform.
#[async_trait]
pub trait ProductScoped<T>: Send + Sync
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    /// Find entity by ID within product scope
    async fn find_by_id_scoped(&self, product_id: &str, id: &str) -> Result<Option<T>>;

    /// Find all entities for a specific product
    async fn find_all_scoped(&self, product_id: &str) -> Result<Vec<T>>;

    /// Create entity scoped to product
    async fn create_scoped(&self, product_id: &str, entity: &T) -> Result<T>;

    /// Update entity scoped to product
    async fn update_scoped(&self, product_id: &str, id: &str, entity: &T) -> Result<T>;

    /// Delete entity scoped to product
    async fn delete_scoped(&self, product_id: &str, id: &str) -> Result<()>;

    /// Count entities for a product
    async fn count_scoped(&self, product_id: &str) -> Result<i64>;
}

/// Pagination parameters
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaginationParams {
    /// Page offset (zero-indexed)
    pub offset: i64,
    /// Number of items per page
    pub limit: i64,
    /// Optional sort field
    pub sort_by: Option<String>,
    /// Sort direction (asc/desc)
    pub sort_desc: bool,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 20,
            sort_by: None,
            sort_desc: false,
        }
    }
}

impl PaginationParams {
    /// Create new pagination params
    pub fn new(offset: i64, limit: i64) -> Self {
        Self {
            offset,
            limit: limit.min(100), // Cap at 100 items
            sort_by: None,
            sort_desc: false,
        }
    }

    /// Set sort field
    pub fn with_sort(mut self, field: impl Into<String>, desc: bool) -> Self {
        self.sort_by = Some(field.into());
        self.sort_desc = desc;
        self
    }

    /// Calculate SQL OFFSET value
    pub fn sql_offset(&self) -> i64 {
        self.offset
    }

    /// Calculate SQL LIMIT value
    pub fn sql_limit(&self) -> i64 {
        self.limit
    }
}

/// Paginated response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: i64, params: &PaginationParams) -> Self {
        let has_more = (params.offset + params.limit) < total;
        Self {
            items,
            total,
            offset: params.offset,
            limit: params.limit,
            has_more,
        }
    }
}

/// Paginated repository trait
#[async_trait]
pub trait PaginatedRepository<T>: Send + Sync
where
    T: Serialize + DeserializeOwned + Send + Sync,
{
    /// Find entities with pagination
    async fn find_paginated(&self, params: &PaginationParams) -> Result<PaginatedResponse<T>>;

    /// Find entities with pagination scoped to product
    async fn find_paginated_scoped(
        &self,
        product_id: &str,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<T>>;
}

/// Base repository implementation
pub struct BaseRepository<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> BaseRepository<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Default for BaseRepository<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize)]
    struct TestEntity {
        id: String,
        name: String,
    }

    #[test]
    fn test_base_repository() {
        let _repo = BaseRepository::<TestEntity>::new();
    }

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams::new(0, 50);
        assert_eq!(params.offset, 0);
        assert_eq!(params.limit, 50);
        assert_eq!(params.sql_offset(), 0);
        assert_eq!(params.sql_limit(), 50);

        // Test limit cap
        let params = PaginationParams::new(0, 200);
        assert_eq!(params.limit, 100); // Capped at 100
    }

    #[test]
    fn test_pagination_params_with_sort() {
        let params = PaginationParams::new(20, 10)
            .with_sort("created_at", true);

        assert_eq!(params.offset, 20);
        assert_eq!(params.limit, 10);
        assert_eq!(params.sort_by, Some("created_at".to_string()));
        assert!(params.sort_desc);
    }

    #[test]
    fn test_pagination_params_defaults() {
        let params = PaginationParams::default();
        assert_eq!(params.offset, 0);
        assert_eq!(params.limit, 20);
        assert_eq!(params.sort_by, None);
        assert!(!params.sort_desc);
    }

    #[test]
    fn test_paginated_response() {
        let items = vec![
            TestEntity { id: "1".to_string(), name: "One".to_string() },
            TestEntity { id: "2".to_string(), name: "Two".to_string() },
        ];
        let params = PaginationParams::new(0, 2);
        let response = PaginatedResponse::new(items, 10, &params);

        assert_eq!(response.items.len(), 2);
        assert_eq!(response.total, 10);
        assert_eq!(response.offset, 0);
        assert_eq!(response.limit, 2);
        assert!(response.has_more); // 0 + 2 < 10

        // Test last page
        let params = PaginationParams::new(8, 2);
        let response: PaginatedResponse<TestEntity> = PaginatedResponse::new(vec![], 10, &params);
        assert!(!response.has_more); // 8 + 2 >= 10
    }
}
