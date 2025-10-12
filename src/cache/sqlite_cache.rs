use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use super::schema;

#[derive(Debug, Clone)]
pub struct SqliteCache {
    pub(super) pool: Arc<Pool<SqliteConnectionManager>>,
    pub(super) instance_id: String,
}

impl SqliteCache {
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let manager = SqliteConnectionManager::file(path).with_init(|conn| {
            // Enable WAL mode for better concurrency
            conn.execute_batch(
                "PRAGMA journal_mode = WAL;
                     PRAGMA synchronous = NORMAL;
                     PRAGMA foreign_keys = ON;
                     PRAGMA busy_timeout = 5000;
                     PRAGMA cache_size = -64000;", // 64MB cache
            )?;
            Ok(())
        });

        let pool = Pool::builder()
            .max_size(10)
            .min_idle(Some(2))
            .connection_timeout(Duration::from_secs(5))
            .build(manager)?;

        let instance_id = uuid::Uuid::new_v4().to_string();

        let cache = Self {
            pool: Arc::new(pool),
            instance_id,
        };

        schema::initialize_schema(&cache.pool).await?;
        Ok(cache)
    }
}
