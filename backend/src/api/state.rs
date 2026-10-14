use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: Option<SqlitePool>,
    pub config: Arc<AppConfig>,
}

impl AppState {
    pub fn new(db: Option<SqlitePool>, config: AppConfig) -> Self {
        Self {
            db,
            config: Arc::new(config),
        }
    }

    pub fn db(&self) -> Result<SqlitePool, crate::errors::AppError> {
        self.db.clone().ok_or_else(|| {
            crate::errors::AppError::Configuration("DATABASE_URL is not configured".to_owned())
        })
    }
}
