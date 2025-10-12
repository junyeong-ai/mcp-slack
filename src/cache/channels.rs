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
