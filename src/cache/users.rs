use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

use crate::slack::types::SlackUser;

use super::sqlite_cache::SqliteCache;

impl SqliteCache {
    // User operations
    pub async fn save_users(&self, users: Vec<SlackUser>) -> Result<()> {
        if users.is_empty() {
            return Err(anyhow::anyhow!("No users to save"));
        }

        self.with_lock("users_update", || {
            let conn = self.pool.get()?;

            // Use temporary table for atomic swap
            conn.execute(
                "CREATE TEMP TABLE IF NOT EXISTS users_new (
                    id TEXT PRIMARY KEY,
                    data TEXT NOT NULL,
                    updated_at INTEGER DEFAULT (unixepoch())
                )",
                [],
            )?;

            // Clear temp table
            conn.execute("DELETE FROM users_new", [])?;

            // Insert new data into temp table
            let tx = conn.unchecked_transaction()?;
            let mut successful_count = 0;

            for user in users {
                if let Ok(json) = serde_json::to_string(&user)
                    && tx.execute(
                        "INSERT INTO users_new (id, data) VALUES (?, ?)",
                        params![&user.id, json],
                    ).is_ok() {
                        successful_count += 1;
                    }
            }

            if successful_count == 0 {
                return Err(anyhow::anyhow!("Failed to save any users"));
            }

            // Atomic swap: delete old and insert from new
            tx.execute("DELETE FROM users", [])?;
            tx.execute("INSERT INTO users (id, data, updated_at) SELECT id, data, updated_at FROM users_new", [])?;
            tx.execute("DELETE FROM users_new", [])?;

            // Update sync timestamp
            let now = Utc::now();
            tx.execute(
                "INSERT OR REPLACE INTO metadata (key, value) VALUES ('last_user_sync', ?)",
                params![serde_json::to_string(&now.to_rfc3339())?],
            )?;

            tx.commit()?;
            Ok(())
        }).await
    }

    pub async fn get_users(&self) -> Result<Vec<SlackUser>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT data FROM users WHERE is_bot = 0 OR is_bot IS NULL ORDER BY name",
        )?;

        let users = stmt
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

        Ok(users)
    }

    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<SlackUser>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT data FROM users WHERE id = ?1")?;

        let result = stmt.query_row(params![user_id], |row| {
            let json: String = row.get(0)?;
            serde_json::from_str(&json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        });

        match result {
            Ok(user) => Ok(Some(user)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn search_users(&self, query: &str, limit: usize) -> Result<Vec<SlackUser>> {
        let conn = self.pool.get()?;

        // Handle empty or special queries
        let processed_query = self.process_fts_query(query);

        if processed_query.is_empty() {
            // Return all users for empty query
            let mut stmt = conn.prepare_cached(
                "SELECT data FROM users WHERE is_bot = 0 OR is_bot IS NULL ORDER BY name LIMIT ?1",
            )?;

            let users = stmt
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

            return Ok(users);
        }

        // Try FTS5 search first
        let fts_result = conn
            .prepare_cached(
                "SELECT u.data
             FROM users u
             JOIN users_fts f ON u.rowid = f.rowid
             WHERE users_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
            )
            .and_then(|mut stmt| {
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
            Ok(users) => Ok(users),
            Err(_) => {
                // Fallback to LIKE search if FTS5 fails
                let mut stmt = conn.prepare_cached(
                    "SELECT data FROM users
                     WHERE (is_bot = 0 OR is_bot IS NULL)
                     AND (name LIKE ?1 OR display_name LIKE ?1 OR real_name LIKE ?1 OR email LIKE ?1)
                     ORDER BY name
                     LIMIT ?2"
                )?;

                let like_query = format!("%{}%", query);
                let users = stmt
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

                Ok(users)
            }
        }
    }
}
