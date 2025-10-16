use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use super::message_utils::format_message;
use super::{IntoToolResponse, Tool, ToolResponse};
use crate::cache::SqliteCache;
use crate::error::{IntoMcpError, McpResult};
use crate::slack::SlackClient;
use crate::utils::parse_params;

pub struct SearchUsersTool {
    cache: Arc<SqliteCache>,
}

pub struct SearchChannelsTool {
    cache: Arc<SqliteCache>,
}

pub struct SearchMessagesTool {
    slack_client: Arc<SlackClient>,
    cache: Arc<SqliteCache>,
}

impl SearchUsersTool {
    pub fn new(cache: Arc<SqliteCache>) -> Self {
        Self { cache }
    }
}

impl SearchChannelsTool {
    pub fn new(cache: Arc<SqliteCache>) -> Self {
        Self { cache }
    }
}

impl SearchMessagesTool {
    pub fn new(slack_client: Arc<SlackClient>, cache: Arc<SqliteCache>) -> Self {
        Self {
            slack_client,
            cache,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SearchUsersParams {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    include_bots: bool,
}

#[derive(Debug, Deserialize)]
struct SearchChannelsParams {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct SearchMessagesParams {
    query: String,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    from_user: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

#[async_trait]
impl Tool for SearchUsersTool {
    fn description(&self) -> &str {
        "Search users by name or email"
    }

    async fn execute(&self, params: Value) -> McpResult<Value> {
        let params: SearchUsersParams = parse_params(params)?;

        let users = self
            .cache
            .search_users(&params.query, params.limit, params.include_bots)
            .mcp_context("Failed to search users")?;

        // Format response with essential user fields
        let user_results: Vec<Value> = users
            .into_iter()
            .map(|user| {
                let mut result = json!({
                    "id": user.id,
                    "name": user.name,
                });

                // Add only essential optional fields
                if user.is_bot {
                    result["is_bot"] = json!(true);
                }
                if let Some(real_name) = user.real_name()
                    && !real_name.is_empty()
                {
                    result["real_name"] = json!(real_name);
                }
                if let Some(display_name) = user.display_name()
                    && !display_name.is_empty()
                    && display_name != user.name
                {
                    // Only include if non-empty and different from name
                    result["display_name"] = json!(display_name);
                }
                if user.deleted {
                    result["deleted"] = json!(true);
                }

                result
            })
            .collect();

        Ok(ToolResponse::data(json!(user_results)).into_response()?)
    }
}

#[async_trait]
impl Tool for SearchChannelsTool {
    fn description(&self) -> &str {
        "Search channels by name"
    }

    async fn execute(&self, params: Value) -> McpResult<Value> {
        let params: SearchChannelsParams = parse_params(params)?;

        let channels = self
            .cache
            .search_channels(&params.query, params.limit)
            .mcp_context("Failed to search channels")?;

        // Format response with useful channel fields
        let channel_results: Vec<Value> = channels
            .into_iter()
            .map(|channel| {
                let mut result = json!({
                    "id": channel.id,
                    "name": channel.name,
                });

                // Only include boolean flags when true (omit false to save tokens)
                if channel.is_private {
                    result["is_private"] = json!(true);
                }
                if channel.is_im {
                    result["is_im"] = json!(true);
                }
                if channel.is_mpim {
                    result["is_mpim"] = json!(true);
                }
                if channel.is_archived {
                    result["is_archived"] = json!(true);
                }
                if channel.is_member {
                    result["is_member"] = json!(true);
                }
                if let Some(num_members) = channel.num_members {
                    result["num_members"] = json!(num_members);
                }

                result
            })
            .collect();

        Ok(ToolResponse::data(json!(channel_results)).into_response()?)
    }
}

#[async_trait]
impl Tool for SearchMessagesTool {
    fn description(&self) -> &str {
        "Search messages (includes threads)"
    }

    async fn execute(&self, params: Value) -> McpResult<Value> {
        let params: SearchMessagesParams = parse_params(params)?;

        // Resolve channel ID to name if needed for search API
        let channel_for_search = if let Some(channel) = &params.channel {
            // If it's a channel ID, resolve to channel name
            if channel.starts_with('C') || channel.starts_with('G') {
                // Try to find channel name from cache
                let channels = self
                    .cache
                    .get_channels()
                    .mcp_context("Failed to get channels from cache")?;

                channels
                    .iter()
                    .find(|c| c.id == *channel)
                    .map(|c| c.name.clone())
                    .or_else(|| Some(channel.clone())) // Fallback to original if not found
            } else {
                Some(channel.clone())
            }
        } else {
            None
        };

        let messages = self
            .slack_client
            .messages
            .search_messages(
                &params.query,
                channel_for_search.as_deref(),
                params.from_user.as_deref(),
                params.limit,
            )
            .await
            .mcp_context("Failed to search messages")?;

        // Format response using common utility
        let mut message_results = Vec::new();
        for msg in messages {
            message_results.push(format_message(msg, &self.cache, true).await);
        }

        Ok(ToolResponse::data(json!(message_results)).into_response()?)
    }
}
