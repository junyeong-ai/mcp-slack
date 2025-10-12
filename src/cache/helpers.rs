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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slack::types::{SlackChannel, SlackUser, SlackUserProfile};
    use rstest::rstest;

    async fn setup_cache() -> SqliteCache {
        SqliteCache::new(":memory:")
            .await
            .expect("Failed to create test cache")
    }

    fn create_test_user(id: &str, name: &str) -> SlackUser {
        SlackUser {
            id: id.to_string(),
            name: name.to_string(),
            is_bot: false,
            is_admin: false,
            deleted: false,
            profile: Some(SlackUserProfile {
                real_name: Some(name.to_string()),
                display_name: Some(name.to_string()),
                email: Some(format!("{}@example.com", name)),
                status_text: None,
                status_emoji: None,
            }),
        }
    }

    fn create_test_channel(id: &str, name: &str) -> SlackChannel {
        SlackChannel {
            id: id.to_string(),
            name: name.to_string(),
            is_channel: true,
            is_private: false,
            is_archived: false,
            is_general: false,
            is_im: false,
            is_mpim: false,
            is_member: true,
            created: None,
            creator: None,
            topic: None,
            purpose: None,
            num_members: Some(10),
        }
    }

    // Tests for process_fts_query

    #[rstest]
    #[case("", "")]
    #[case("   ", "")]
    #[case("*", "")]
    #[case("%", "")]
    #[case("simple", "\"simple\"")]
    #[case("hello world", "\"hello world\"")]
    #[case("test*query", "\"testquery\"")]
    #[case("user%name", "\"username\"")]
    #[case("  padded  ", "\"padded\"")]
    fn test_process_fts_query(#[case] input: &str, #[case] expected: &str) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = rt.block_on(setup_cache());
        let result = cache.process_fts_query(input);
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_process_fts_query_escapes_quotes() {
        let cache = setup_cache().await;
        let result = cache.process_fts_query("test\"query");
        assert_eq!(result, "\"test\"\"query\"");
    }

    #[tokio::test]
    async fn test_process_fts_query_multiple_special_chars() {
        let cache = setup_cache().await;
        let result = cache.process_fts_query("*test%query*");
        assert_eq!(result, "\"testquery\"");
    }

    #[tokio::test]
    async fn test_process_fts_query_only_special_chars() {
        let cache = setup_cache().await;
        let result = cache.process_fts_query("***%%%");
        assert_eq!(result, "");
    }

    // Tests for is_cache_stale

    #[tokio::test]
    async fn test_is_cache_stale_empty_cache() {
        let cache = setup_cache().await;
        let result = cache.is_cache_stale(None).await.unwrap();
        // Empty cache should be considered stale
        assert!(result);
    }

    #[tokio::test]
    async fn test_is_cache_stale_fresh_cache() {
        let cache = setup_cache().await;

        // Save users and channels to populate metadata
        let users = vec![create_test_user("U123", "alice")];
        cache.save_users(users).await.unwrap();

        let channels = vec![create_test_channel("C123", "general")];
        cache.save_channels(channels).await.unwrap();

        let result = cache.is_cache_stale(Some(24)).await.unwrap();
        // Freshly saved cache should not be stale
        assert!(!result);
    }

    #[tokio::test]
    async fn test_is_cache_stale_with_default_ttl() {
        let cache = setup_cache().await;

        let users = vec![create_test_user("U123", "alice")];
        cache.save_users(users).await.unwrap();

        let channels = vec![create_test_channel("C123", "general")];
        cache.save_channels(channels).await.unwrap();

        // Use default TTL (24 hours)
        let result = cache.is_cache_stale(None).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_is_cache_stale_with_custom_ttl() {
        let cache = setup_cache().await;

        let users = vec![create_test_user("U123", "alice")];
        cache.save_users(users).await.unwrap();

        let channels = vec![create_test_channel("C123", "general")];
        cache.save_channels(channels).await.unwrap();

        // Use very short TTL (0 hours) - should be immediately stale
        let result = cache.is_cache_stale(Some(0)).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_is_cache_stale_partial_data() {
        let cache = setup_cache().await;

        // Only save users, not channels
        let users = vec![create_test_user("U123", "alice")];
        cache.save_users(users).await.unwrap();

        let result = cache.is_cache_stale(None).await.unwrap();
        // Should be stale because channels are missing
        assert!(result);
    }

    // Tests for get_counts

    #[tokio::test]
    async fn test_get_counts_empty_cache() {
        let cache = setup_cache().await;
        let (user_count, channel_count) = cache.get_counts().await.unwrap();
        assert_eq!(user_count, 0);
        assert_eq!(channel_count, 0);
    }

    #[tokio::test]
    async fn test_get_counts_with_users() {
        let cache = setup_cache().await;

        let users = vec![
            create_test_user("U123", "alice"),
            create_test_user("U456", "bob"),
            create_test_user("U789", "charlie"),
        ];
        cache.save_users(users).await.unwrap();

        let (user_count, channel_count) = cache.get_counts().await.unwrap();
        assert_eq!(user_count, 3);
        assert_eq!(channel_count, 0);
    }

    #[tokio::test]
    async fn test_get_counts_with_channels() {
        let cache = setup_cache().await;

        let channels = vec![
            create_test_channel("C123", "general"),
            create_test_channel("C456", "random"),
        ];
        cache.save_channels(channels).await.unwrap();

        let (user_count, channel_count) = cache.get_counts().await.unwrap();
        assert_eq!(user_count, 0);
        assert_eq!(channel_count, 2);
    }

    #[tokio::test]
    async fn test_get_counts_with_both() {
        let cache = setup_cache().await;

        let users = vec![
            create_test_user("U123", "alice"),
            create_test_user("U456", "bob"),
        ];
        cache.save_users(users).await.unwrap();

        let channels = vec![
            create_test_channel("C123", "general"),
            create_test_channel("C456", "random"),
            create_test_channel("C789", "announcements"),
        ];
        cache.save_channels(channels).await.unwrap();

        let (user_count, channel_count) = cache.get_counts().await.unwrap();
        assert_eq!(user_count, 2);
        assert_eq!(channel_count, 3);
    }

    #[tokio::test]
    async fn test_get_counts_after_updates() {
        let cache = setup_cache().await;

        // Initial save
        let users = vec![create_test_user("U123", "alice")];
        cache.save_users(users).await.unwrap();

        let (user_count, _channel_count) = cache.get_counts().await.unwrap();
        assert_eq!(user_count, 1);

        // Update with more users (atomic swap)
        let users = vec![
            create_test_user("U123", "alice"),
            create_test_user("U456", "bob"),
            create_test_user("U789", "charlie"),
        ];
        cache.save_users(users).await.unwrap();

        let (user_count, channel_count) = cache.get_counts().await.unwrap();
        assert_eq!(user_count, 3);
        assert_eq!(channel_count, 0);
    }
}
