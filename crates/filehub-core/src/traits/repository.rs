//! Generic repository trait for database access.

use async_trait::async_trait;

use crate::result::AppResult;
use crate::types::pagination::{PageRequest, PageResponse};

/// Generic CRUD repository trait.
///
/// This trait is defined with generic type parameters so that each
/// entity can have a strongly typed repository. Entity-specific
/// query methods are defined on the concrete repository structs.
#[async_trait]
pub trait Repository<Entity, Id>: Send + Sync + 'static
where
    Entity: Send + Sync + 'static + serde::Serialize,
    Id: Send + Sync + 'static,
{
    /// Find an entity by its primary key.
    async fn find_by_id(&self, id: &Id) -> AppResult<Option<Entity>>;

    /// Find all entities with pagination.
    async fn find_all(&self, page: &PageRequest) -> AppResult<PageResponse<Entity>>;

    /// Create a new entity and return it.
    async fn create(&self, entity: &Entity) -> AppResult<Entity>;

    /// Update an existing entity and return the updated version.
    async fn update(&self, entity: &Entity) -> AppResult<Entity>;

    /// Delete an entity by its primary key. Returns `true` if deleted.
    async fn delete(&self, id: &Id) -> AppResult<bool>;

    /// Count total entities.
    async fn count(&self) -> AppResult<u64>;
}
