use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::params;
use tracing::warn;

use super::sqlite_cache::SqliteCache;

const LOCK_TIMEOUT_SECS: u64 = 60;
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 500;

impl SqliteCache {
    // Lock management for multi-instance coordination
    pub(super) async fn acquire_lock(&self, key: &str) -> Result<()> {
        let mut backoff = Duration::from_millis(INITIAL_BACKOFF_MS);

        for attempt in 0..MAX_RETRIES {
            let conn = self.pool.get()?;
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
            let expires_at = now + LOCK_TIMEOUT_SECS as i64;

            // Clean up expired locks
            conn.execute("DELETE FROM locks WHERE expires_at < ?", params![now])?;

            // Try to acquire lock
            let result = conn.execute(
                "INSERT INTO locks (key, instance_id, acquired_at, expires_at) VALUES (?, ?, ?, ?)",
                params![key, &self.instance_id, now, expires_at],
            );

            match result {
                Ok(_) => {
                    return Ok(());
                }
                Err(rusqlite::Error::SqliteFailure(err, _))
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    // Lock is held by another instance
                    if attempt < MAX_RETRIES - 1 {
                        // Check if lock is stale (held by dead instance)
                        if let Ok(lock_info) = conn.query_row(
                            "SELECT instance_id, acquired_at FROM locks WHERE key = ?",
                            params![key],
                            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                        ) {
                            let (holder_id, acquired_at) = lock_info;
                            let lock_age = now - acquired_at;

                            // If lock is very old, it might be from a dead instance
                            if lock_age > (LOCK_TIMEOUT_SECS * 2) as i64 {
                                warn!(
                                    "Detected potentially stale lock held by {} for {} seconds, forcing cleanup",
                                    holder_id, lock_age
                                );
                                // Force delete the stale lock
                                let _ = conn.execute(
                                    "DELETE FROM locks WHERE key = ? AND instance_id = ?",
                                    params![key, holder_id],
                                );
                                continue; // Retry immediately
                            }
                        }

                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(Duration::from_secs(1));
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }

        Err(anyhow::anyhow!(
            "Failed to acquire lock after {} attempts",
            MAX_RETRIES
        ))
    }

    pub(super) async fn release_lock(&self, key: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM locks WHERE key = ? AND instance_id = ?",
            params![key, &self.instance_id],
        )?;
        Ok(())
    }

    pub async fn with_lock<F, R>(&self, key: &str, f: F) -> Result<R>
    where
        F: FnOnce() -> Result<R>,
    {
        self.acquire_lock(key).await?;

        // Execute function and always try to release lock, even if function fails
        let result = f();

        // Try to release lock, but don't fail if release fails
        // Lock will expire automatically after timeout
        if let Err(e) = self.release_lock(key).await {
            warn!(
                "Failed to release lock for key '{}': {}. Lock will expire automatically.",
                key, e
            );
        }

        result
    }
}
