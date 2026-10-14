use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::application::timeline::PlaceNameResolver;
use crate::domain::Coordinate;
use crate::errors::AppError;

pub struct CachedPlaceNames<R> {
    inner: R,
    path: PathBuf,
    entries: Mutex<HashMap<String, String>>,
}

impl<R> CachedPlaceNames<R> {
    pub async fn load(inner: R, path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let entries = tokio::fs::read(&path)
            .await
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();

        Self {
            inner,
            path,
            entries: Mutex::new(entries),
        }
    }

    async fn persist(&self, entries: &HashMap<String, String>) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| AppError::Internal(error.to_string()))?;
        }
        let payload = serde_json::to_vec_pretty(entries)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        tokio::fs::write(&self.path, payload)
            .await
            .map_err(|error| AppError::Internal(error.to_string()))
    }
}

fn cache_key(coordinate: Coordinate) -> String {
    format!("{:.6},{:.6}", coordinate.latitude(), coordinate.longitude())
}

#[async_trait]
impl<R> PlaceNameResolver for CachedPlaceNames<R>
where
    R: PlaceNameResolver,
{
    async fn resolve(&self, coordinate: Coordinate) -> Result<Option<String>, AppError> {
        let key = cache_key(coordinate);
        if let Some(cached) = self.entries.lock().await.get(&key) {
            return Ok(Some(cached.clone()));
        }

        let resolved = self.inner.resolve(coordinate).await?;
        if let Some(display_name) = &resolved {
            let snapshot = {
                let mut entries = self.entries.lock().await;
                entries.insert(key, display_name.clone());
                entries.clone()
            };
            self.persist(&snapshot).await?;
        }

        Ok(resolved)
    }
}
