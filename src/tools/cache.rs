use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use super::{IntoToolResponse, Tool, ToolResponse};
use crate::cache::{CacheRefreshType, SqliteCache};
use crate::error::{McpError, McpResult};
use crate::slack::SlackClient;
use crate::utils::parse_params;

pub struct RefreshCacheTool {
    slack_client: Arc<SlackClient>,
    cache: Arc<SqliteCache>,
}

impl RefreshCacheTool {
    pub fn new(slack_client: Arc<SlackClient>, cache: Arc<SqliteCache>) -> Self {
        Self {
            slack_client,
            cache,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RefreshCacheParams {
    #[serde(default = "default_all")]
    refresh_type: String,
}

fn default_all() -> String {
    "all".to_string()
}

#[async_trait]
impl Tool for RefreshCacheTool {
    fn description(&self) -> &str {
        "Refresh cached data (users/channels/all)"
    }

    async fn execute(&self, params: Value) -> McpResult<Value> {
        // Parse parameters with default values
        let params: RefreshCacheParams = parse_params(params).unwrap_or(RefreshCacheParams {
            refresh_type: "all".to_string(),
        });

        // Determine refresh type
        let refresh_type = match params.refresh_type.as_str() {
            "users" => CacheRefreshType::Users,
            "channels" => CacheRefreshType::Channels,
            "all" => CacheRefreshType::All,
            _ => CacheRefreshType::All,
        };

        // Check if cache needs refreshing (with minimal race condition window)
        let (user_count, channel_count) = self.cache.get_counts().unwrap_or((0, 0));
        let is_stale = self.cache.is_cache_stale(Some(1)).unwrap_or(true);

        // Check cache status

        let mut refreshed_users = false;
        let mut refreshed_channels = false;
        let mut errors = Vec::new();

        // Force refresh if cache is empty, regardless of stale status
        if is_stale || (user_count == 0 && channel_count == 0) {
            // Perform refresh without lock but with short TTL check
            match refresh_type {
                CacheRefreshType::Users | CacheRefreshType::All => {
                    match self.slack_client.users.fetch_all_users().await {
                        Ok(users) => {
                            if let Err(e) = self.cache.save_users(users).await {
                                let error_msg = format!("Failed to save users: {}", e);
                                errors.push(error_msg);
                            } else {
                                refreshed_users = true;
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to fetch users: {}", e);
                            errors.push(error_msg);
                        }
                    }
                }
                _ => {}
            }

            match refresh_type {
                CacheRefreshType::Channels | CacheRefreshType::All => {
                    match self.slack_client.channels.fetch_all_channels().await {
                        Ok(channels) => {
                            if let Err(e) = self.cache.save_channels(channels).await {
                                let error_msg = format!("Failed to save channels: {}", e);
                                errors.push(error_msg);
                            } else {
                                refreshed_channels = true;
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to fetch channels: {}", e);
                            errors.push(error_msg);
                        }
                    }
                }
                _ => {}
            }

            if !errors.is_empty() {
                return Err(McpError::Internal(errors.join("; ")));
            }
        } else {
            // Cache is already fresh, skipping refresh
        }

        Ok(ToolResponse::data(json!({
            "refreshed": true,
            "type": params.refresh_type,
            "users_refreshed": refreshed_users,
            "channels_refreshed": refreshed_channels,
        }))
        .into_response()?)
    }
}
