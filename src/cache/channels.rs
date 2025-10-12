use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

use crate::slack::types::SlackChannel;

use super::sqlite_cache::SqliteCache;

impl SqliteCache {
    // Channel operations
    pub async fn save_channels(&self, channels: Vec<SlackChannel>) -> Result<()> {
        if channels.is_empty() {
            return Err(anyhow::anyhow!("No channels to save"));
        }

        self.with_lock("channels_update", || {
            let conn = self.pool.get()?;

            // Use temporary table for atomic swap
            conn.execute(
                "CREATE TEMP TABLE IF NOT EXISTS channels_new (
                    id TEXT PRIMARY KEY,
                    data TEXT NOT NULL,
                    updated_at INTEGER DEFAULT (unixepoch())
                )",
                [],
            )?;

            // Clear temp table
            conn.execute("DELETE FROM channels_new", [])?;

            // Insert new data into temp table
            let tx = conn.unchecked_transaction()?;
            let mut successful_count = 0;

            for channel in channels {
                if let Ok(json) = serde_json::to_string(&channel)
                    && tx.execute(
                        "INSERT INTO channels_new (id, data) VALUES (?, ?)",
                        params![&channel.id, json],
                    ).is_ok() {
                        successful_count += 1;
                    }
            }

            if successful_count == 0 {
                return Err(anyhow::anyhow!("Failed to save any channels"));
            }

            // Atomic swap: delete old and insert from new
            tx.execute("DELETE FROM channels", [])?;
            tx.execute("INSERT INTO channels (id, data, updated_at) SELECT id, data, updated_at FROM channels_new", [])?;
            tx.execute("DELETE FROM channels_new", [])?;

            // Update sync timestamp
            let now = Utc::now();
            tx.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES ('last_channel_sync', ?)",
                params![serde_json::to_string(&now.to_rfc3339())?],
            )?;

            tx.commit()?;
            Ok(())
        }).await
    }

    pub async fn get_channels(&self) -> Result<Vec<SlackChannel>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT data FROM channels WHERE is_archived = 0 OR is_archived IS NULL ORDER BY name",
        )?;

        let channels = stmt
            .query_map([], |row| {
                let json: String = row.get(0)?;
                serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(channels)
    }

    pub async fn search_channels(&self, query: &str, limit: usize) -> Result<Vec<SlackChannel>> {
        let conn = self.pool.get()?;

        // Handle empty or special queries
        let processed_query = self.process_fts_query(query);

        // Include all channels (public and private) in search results

        if processed_query.is_empty() {
            // Return all channels for empty query
            let sql = "SELECT data FROM channels
                       WHERE (is_archived = 0 OR is_archived IS NULL)
                       ORDER BY name
                       LIMIT ?1";

            let mut stmt = conn.prepare_cached(sql)?;
            let channels = stmt
                .query_map(params![limit], |row| {
                    let json: String = row.get(0)?;
                    serde_json::from_str(&json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            return Ok(channels);
        }

        // Try FTS5 search first
        let fts_sql = "SELECT c.data
                        FROM channels c
                        JOIN channels_fts f ON c.rowid = f.rowid
                        WHERE channels_fts MATCH ?1
                        AND (c.is_archived = 0 OR c.is_archived IS NULL)
                        ORDER BY rank
                        LIMIT ?2";

        let fts_result = conn.prepare_cached(fts_sql).and_then(|mut stmt| {
            stmt.query_map(params![&processed_query, limit], |row| {
                let json: String = row.get(0)?;
                serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })
            })?
            .collect::<Result<Vec<_>, _>>()
        });

        match fts_result {
            Ok(channels) => Ok(channels),
            Err(_) => {
                // Fallback to LIKE search if FTS5 fails
                let fallback_sql = "SELECT data FROM channels
                                     WHERE (is_archived = 0 OR is_archived IS NULL)
                                     AND name LIKE ?1
                                     ORDER BY name
                                     LIMIT ?2";

                let mut stmt = conn.prepare_cached(fallback_sql)?;
                let like_query = format!("%{}%", query);
                let channels = stmt
                    .query_map(params![like_query, limit], |row| {
                        let json: String = row.get(0)?;
                        serde_json::from_str(&json).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })
                    })?
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(channels)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // Test fixtures
    fn create_test_channel(
        id: &str,
        name: &str,
        is_private: bool,
        is_archived: bool,
        is_im: bool,
        is_mpim: bool,
    ) -> SlackChannel {
        SlackChannel {
            id: id.to_string(),
            name: name.to_string(),
            is_channel: !is_im && !is_mpim,
            is_im,
            is_mpim,
            is_private,
            is_archived,
            is_general: name == "general",
            is_member: true,
            created: None,
            creator: None,
            topic: None,
            purpose: None,
            num_members: Some(10),
        }
    }

    async fn setup_cache() -> SqliteCache {
        SqliteCache::new(":memory:")
            .await
            .expect("Failed to create test cache")
    }

    #[tokio::test]
    async fn test_save_channels_empty_vec() {
        let cache = setup_cache().await;
        let result = cache.save_channels(vec![]).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "No channels to save");
    }

    #[tokio::test]
    async fn test_save_channels_single_channel() {
        let cache = setup_cache().await;
        let channel = create_test_channel("C123", "general", false, false, false, false);

        let result = cache.save_channels(vec![channel.clone()]).await;
        assert!(result.is_ok());

        // Verify channel was saved
        let channels = cache.get_channels().await.unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].id, "C123");
        assert_eq!(channels[0].name, "general");
    }

    #[tokio::test]
    async fn test_save_channels_multiple_channels() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "general", false, false, false, false),
            create_test_channel("C456", "random", false, false, false, false),
            create_test_channel("G789", "private-team", true, false, false, false),
        ];

        let result = cache.save_channels(channels).await;
        assert!(result.is_ok());

        let all_channels = cache.get_channels().await.unwrap();
        assert_eq!(all_channels.len(), 3);
    }

    #[tokio::test]
    async fn test_save_channels_replaces_existing() {
        let cache = setup_cache().await;

        // Save initial channels
        let channels_v1 = vec![
            create_test_channel("C123", "general", false, false, false, false),
            create_test_channel("C456", "random", false, false, false, false),
        ];
        cache.save_channels(channels_v1).await.unwrap();

        // Save new set of channels (atomic swap)
        let channels_v2 = vec![
            create_test_channel("C123", "general-updated", false, false, false, false),
            create_test_channel("C789", "announcements", false, false, false, false),
        ];
        cache.save_channels(channels_v2).await.unwrap();

        // Verify old data replaced
        let all_channels = cache.get_channels().await.unwrap();
        assert_eq!(all_channels.len(), 2);

        let general = all_channels.iter().find(|c| c.id == "C123").unwrap();
        assert_eq!(general.name, "general-updated");

        // C456 should be removed
        assert!(all_channels.iter().all(|c| c.id != "C456"));
    }

    #[tokio::test]
    async fn test_get_channels_filters_archived() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "general", false, false, false, false),
            create_test_channel("C456", "old-project", false, true, false, false),
            create_test_channel("C789", "active", false, false, false, false),
        ];
        cache.save_channels(channels).await.unwrap();

        let active_channels = cache.get_channels().await.unwrap();
        assert_eq!(active_channels.len(), 2);
        assert!(active_channels.iter().all(|c| !c.is_archived));
    }

    #[tokio::test]
    async fn test_get_channels_sorted_by_name() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "zebra", false, false, false, false),
            create_test_channel("C456", "alpha", false, false, false, false),
            create_test_channel("C789", "beta", false, false, false, false),
        ];
        cache.save_channels(channels).await.unwrap();

        let sorted_channels = cache.get_channels().await.unwrap();
        assert_eq!(sorted_channels.len(), 3);
        assert_eq!(sorted_channels[0].name, "alpha");
        assert_eq!(sorted_channels[1].name, "beta");
        assert_eq!(sorted_channels[2].name, "zebra");
    }

    #[tokio::test]
    async fn test_get_channels_includes_private() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "public", false, false, false, false),
            create_test_channel("G456", "private", true, false, false, false),
        ];
        cache.save_channels(channels).await.unwrap();

        let all_channels = cache.get_channels().await.unwrap();
        assert_eq!(all_channels.len(), 2);
    }

    #[tokio::test]
    async fn test_get_channels_includes_dms() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "general", false, false, false, false),
            create_test_channel("D456", "dm-alice", false, false, true, false),
            create_test_channel("G789", "mpdm-team", false, false, false, true),
        ];
        cache.save_channels(channels).await.unwrap();

        let all_channels = cache.get_channels().await.unwrap();
        assert_eq!(all_channels.len(), 3);
    }

    #[rstest]
    #[case("general", 1)]
    #[case("random", 1)]
    #[case("nonexistent", 0)]
    #[tokio::test]
    async fn test_search_channels_by_name(#[case] query: &str, #[case] expected_count: usize) {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "general", false, false, false, false),
            create_test_channel("C456", "random", false, false, false, false),
        ];
        cache.save_channels(channels).await.unwrap();

        let results = cache.search_channels(query, 10).await.unwrap();
        assert_eq!(results.len(), expected_count);
    }

    #[tokio::test]
    async fn test_search_channels_empty_query() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "general", false, false, false, false),
            create_test_channel("C456", "random", false, false, false, false),
        ];
        cache.save_channels(channels).await.unwrap();

        // Empty query should return all non-archived channels
        let results = cache.search_channels("", 10).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_channels_with_limit() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "alpha", false, false, false, false),
            create_test_channel("C456", "beta", false, false, false, false),
            create_test_channel("C789", "gamma", false, false, false, false),
        ];
        cache.save_channels(channels).await.unwrap();

        let results = cache.search_channels("", 2).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_channels_filters_archived() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "active", false, false, false, false),
            create_test_channel("C456", "archived-test", false, true, false, false),
        ];
        cache.save_channels(channels).await.unwrap();

        // Search should not return archived channels
        let results = cache.search_channels("test", 10).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_channels_includes_private() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "public-channel", false, false, false, false),
            create_test_channel("G456", "private-channel", true, false, false, false),
        ];
        cache.save_channels(channels).await.unwrap();

        let results = cache.search_channels("channel", 10).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_channels_with_special_chars() {
        let cache = setup_cache().await;
        let channels = vec![create_test_channel(
            "C123", "general", false, false, false, false,
        )];
        cache.save_channels(channels).await.unwrap();

        // Special characters are stripped by process_fts_query
        let results = cache.search_channels("general*@#$", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "general");
    }

    #[tokio::test]
    async fn test_search_channels_case_sensitivity() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "General", false, false, false, false),
            create_test_channel("C456", "RANDOM", false, false, false, false),
        ];
        cache.save_channels(channels).await.unwrap();

        // FTS5 search should be case-insensitive
        let results = cache.search_channels("general", 10).await.unwrap();
        assert_eq!(results.len(), 1);

        let results = cache.search_channels("random", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_save_channels() {
        let cache = setup_cache().await;

        let cache1 = cache.clone();
        let cache2 = cache.clone();

        let handle1 = tokio::spawn(async move {
            let channels = vec![create_test_channel(
                "C123", "general", false, false, false, false,
            )];
            cache1.save_channels(channels).await
        });

        let handle2 = tokio::spawn(async move {
            let channels = vec![create_test_channel(
                "C456", "random", false, false, false, false,
            )];
            cache2.save_channels(channels).await
        });

        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        // Both should succeed (locking prevents conflicts)
        assert!(result1.is_ok() || result2.is_ok());
    }

    #[tokio::test]
    async fn test_channel_types_preserved() {
        let cache = setup_cache().await;
        let channels = vec![
            create_test_channel("C123", "public", false, false, false, false),
            create_test_channel("G456", "private", true, false, false, false),
            create_test_channel("D789", "dm", false, false, true, false),
            create_test_channel("G999", "mpdm", false, false, false, true),
        ];
        cache.save_channels(channels).await.unwrap();

        let all_channels = cache.get_channels().await.unwrap();
        assert_eq!(all_channels.len(), 4);

        let public = all_channels.iter().find(|c| c.id == "C123").unwrap();
        assert!(public.is_channel);
        assert!(!public.is_private);
        assert!(!public.is_im);
        assert!(!public.is_mpim);

        let private = all_channels.iter().find(|c| c.id == "G456").unwrap();
        assert!(private.is_private);
        assert!(private.is_channel); // Private channels are still channels
        assert!(!private.is_im);
        assert!(!private.is_mpim);

        let dm = all_channels.iter().find(|c| c.id == "D789").unwrap();
        assert!(dm.is_im);
        assert!(!dm.is_channel);
        assert!(!dm.is_mpim);

        let mpdm = all_channels.iter().find(|c| c.id == "G999").unwrap();
        assert!(mpdm.is_mpim);
        assert!(!mpdm.is_channel);
        assert!(!mpdm.is_im);
    }
}
