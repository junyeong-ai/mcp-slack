use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;

use super::sqlite_cache::SqliteCache;

const DEFAULT_CACHE_TTL_HOURS: i64 = 24;

impl SqliteCache {
    pub(super) fn process_fts_query(&self, query: &str) -> String {
        let trimmed = query.trim();

        // Handle empty queries
        if trimmed.is_empty() {
            return String::new();
        }

        // Handle wildcard-only queries
        if trimmed == "*" || trimmed == "%" {
            return String::new();
        }

        // Escape and clean FTS5 special characters
        let cleaned = trimmed
            .replace("\"", "\"\"") // Escape quotes
            .replace("*", "") // Remove wildcards
            .replace("%", "") // Remove SQL wildcards
            .trim()
            .to_string();

        if cleaned.is_empty() {
            return String::new();
        }

        // Return as phrase search for better results
        format!("\"{}\"", cleaned)
    }

    pub async fn is_cache_stale(&self, ttl_hours: Option<i64>) -> Result<bool> {
        let conn = self.pool.get()?;
        let ttl_hours = ttl_hours.unwrap_or(DEFAULT_CACHE_TTL_HOURS);
        let stale_threshold = Utc::now() - chrono::Duration::hours(ttl_hours);

        let user_sync_time: Option<String> = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'last_user_sync'",
                [],
                |row| row.get(0),
            )
            .optional()?;

        let channel_sync_time: Option<String> = conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'last_channel_sync'",
                [],
                |row| row.get(0),
            )
            .optional()?;

        let user_stale = match user_sync_time {
            Some(time_str) => {
                let time_str = time_str.trim_matches('"');
                match DateTime::parse_from_rfc3339(time_str) {
                    Ok(dt) => dt.with_timezone(&Utc) < stale_threshold,
                    Err(_) => true,
                }
            }
            None => true,
        };

        let channel_stale = match channel_sync_time {
            Some(time_str) => {
                let time_str = time_str.trim_matches('"');
                match DateTime::parse_from_rfc3339(time_str) {
                    Ok(dt) => dt.with_timezone(&Utc) < stale_threshold,
                    Err(_) => true,
                }
            }
            None => true,
        };

        Ok(user_stale || channel_stale)
    }

    pub async fn get_counts(&self) -> Result<(usize, usize)> {
        let conn = self.pool.get()?;

        let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;

        let channel_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM channels", [], |row| row.get(0))?;

        Ok((user_count as usize, channel_count as usize))
    }
}
