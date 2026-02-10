//! Storage manager — routes operations to the correct provider by storage ID.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use filehub_core::error::AppError;
use filehub_core::result::AppResult;
use filehub_core::traits::storage::StorageProvider;

/// Central storage manager that holds references to all registered providers.
#[derive(Debug, Clone)]
pub struct StorageManager {
    /// Map of storage ID → provider instance.
    providers: Arc<RwLock<HashMap<Uuid, Arc<dyn StorageProvider>>>>,
    /// The default storage ID.
    default_id: Arc<RwLock<Option<Uuid>>>,
}

impl StorageManager {
    /// Create a new empty storage manager.
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            default_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Register a storage provider.
    pub async fn register(
        &self,
        storage_id: Uuid,
        provider: Arc<dyn StorageProvider>,
        is_default: bool,
    ) {
        let mut providers = self.providers.write().await;
        providers.insert(storage_id, provider);
        if is_default {
            let mut default = self.default_id.write().await;
            *default = Some(storage_id);
        }
    }

    /// Remove a storage provider.
    pub async fn unregister(&self, storage_id: &Uuid) {
        let mut providers = self.providers.write().await;
        providers.remove(storage_id);
        let mut default = self.default_id.write().await;
        if default.as_ref() == Some(storage_id) {
            *default = None;
        }
    }

    /// Get a provider by storage ID.
    pub async fn get(&self, storage_id: &Uuid) -> AppResult<Arc<dyn StorageProvider>> {
        let providers = self.providers.read().await;
        providers
            .get(storage_id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("Storage provider {storage_id} not found")))
    }

    /// Get the default storage provider.
    pub async fn get_default(&self) -> AppResult<(Uuid, Arc<dyn StorageProvider>)> {
        let default_id = {
            let default = self.default_id.read().await;
            default.ok_or_else(|| AppError::configuration("No default storage configured"))?
        };
        let provider = self.get(&default_id).await?;
        Ok((default_id, provider))
    }

    /// List all registered storage IDs.
    pub async fn list_ids(&self) -> Vec<Uuid> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }

    /// Check health of all registered providers.
    pub async fn health_check_all(&self) -> HashMap<Uuid, bool> {
        let providers = self.providers.read().await;
        let mut results = HashMap::new();
        for (id, provider) in providers.iter() {
            let healthy = provider.health_check().await.unwrap_or(false);
            results.insert(*id, healthy);
        }
        results
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}
