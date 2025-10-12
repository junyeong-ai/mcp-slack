use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use super::message_utils::{format_message, format_thread_messages};
use super::{IntoToolResponse, Tool, ToolResponse};
use crate::cache::SqliteCache;
use crate::error::{IntoMcpError, McpResult};
use crate::slack::SlackClient;
use crate::utils::{parse_params, resolve_channel_id, validate_required_one_of};

pub struct SendMessageTool {
    slack_client: Arc<SlackClient>,
    cache: Arc<SqliteCache>,
}

pub struct ReadThreadTool {
    slack_client: Arc<SlackClient>,
    cache: Arc<SqliteCache>,
}

pub struct ListChannelMembersTool {
    slack_client: Arc<SlackClient>,
    cache: Arc<SqliteCache>,
}

pub struct GetChannelMessagesTool {
    slack_client: Arc<SlackClient>,
    cache: Arc<SqliteCache>,
}

impl SendMessageTool {
    pub fn new(slack_client: Arc<SlackClient>, cache: Arc<SqliteCache>) -> Self {
        Self {
            slack_client,
            cache,
        }
    }
}

impl ReadThreadTool {
    pub fn new(slack_client: Arc<SlackClient>, cache: Arc<SqliteCache>) -> Self {
        Self {
            slack_client,
            cache,
        }
    }
}

impl ListChannelMembersTool {
    pub fn new(slack_client: Arc<SlackClient>, cache: Arc<SqliteCache>) -> Self {
        Self {
            slack_client,
            cache,
        }
    }
}

impl GetChannelMessagesTool {
    pub fn new(slack_client: Arc<SlackClient>, cache: Arc<SqliteCache>) -> Self {
        Self {
            slack_client,
            cache,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SendMessageParams {
    channel: String,
    text: Option<String>,
    blocks: Option<Value>,
    thread_ts: Option<String>,
    reply_broadcast: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ReadThreadParams {
    channel: String,
    thread_ts: String,
    #[serde(default = "retrieval_default_limit")]
    limit: usize,
}

fn retrieval_default_limit() -> usize {
    100 // Sufficient default for data retrieval operations
}

#[derive(Debug, Deserialize)]
struct ListChannelMembersParams {
    channel: String,
    #[serde(default = "retrieval_default_limit")]
    limit: usize,
}

#[async_trait]
impl Tool for SendMessageTool {
    fn description(&self) -> &str {
        "Send message to channel or DM"
    }

    async fn execute(&self, params: Value) -> McpResult<Value> {
        // Parse parameters
        let params: SendMessageParams = parse_params(params)?;

        // Validate that either text or blocks is provided
        validate_required_one_of(&params.text, &params.blocks, "'text' or 'blocks'")?;

        // Resolve channel ID if name is provided
        let channel_id =
            resolve_channel_id(&params.channel, &self.cache, Some(&self.slack_client)).await?;

        // Send the message
        let blocks_vec: Option<Vec<serde_json::Value>> = params.blocks.map(|b| vec![b]);
        let timestamp = self
            .slack_client
            .messages
            .post_message(
                &channel_id,
                params.text.as_deref(),
                blocks_vec.as_ref(),
                params.thread_ts.as_deref(),
                params.reply_broadcast.unwrap_or(false),
            )
            .await
            .mcp_context("Failed to send message")?;

        Ok(ToolResponse::data(json!({
            "channel": channel_id,
            "ts": timestamp,
        }))
        .into_response()?)
    }
}

#[async_trait]
impl Tool for ReadThreadTool {
    fn description(&self) -> &str {
        "Read all thread messages"
    }

    async fn execute(&self, params: Value) -> McpResult<Value> {
        // Parse parameters
        let params: ReadThreadParams = parse_params(params)?;

        // Resolve channel ID if name is provided
        let channel_id = resolve_channel_id(
            &params.channel,
            &self.cache,
            None, // No slack_client needed for this tool
        )
        .await?;

        // Get thread replies
        let (messages, has_more) = self
            .slack_client
            .messages
            .get_thread_replies(&channel_id, &params.thread_ts, params.limit)
            .await
            .mcp_context("Failed to read thread")?;

        // Use the common formatting utility
        let result = format_thread_messages(messages, &self.cache).await;

        Ok(ToolResponse::paginated(result, has_more, None).into_response()?)
    }
}

#[derive(Debug, Deserialize)]
struct GetChannelMessagesParams {
    channel: String,
    #[serde(default = "retrieval_default_limit")]
    limit: usize,
    #[serde(default)]
    cursor: Option<String>,
}

#[async_trait]
impl Tool for GetChannelMessagesTool {
    fn description(&self) -> &str {
        "Get channel messages (excludes threads)"
    }

    async fn execute(&self, params: Value) -> McpResult<Value> {
        let params: GetChannelMessagesParams = parse_params(params)?;

        // Resolve channel ID if name is provided
        let channel_id = resolve_channel_id(
            &params.channel,
            &self.cache,
            None, // No slack_client needed for this tool
        )
        .await?;

        let (messages, next_cursor) = self
            .slack_client
            .messages
            .get_channel_messages(&channel_id, params.limit, params.cursor.as_deref())
            .await
            .mcp_context("Failed to get channel messages")?;

        // Format response using common utility
        let mut message_results = Vec::new();
        for msg in messages {
            message_results.push(format_message(msg, &self.cache, true).await);
        }

        Ok(ToolResponse::paginated(
            json!({"messages": message_results}),
            next_cursor.is_some(),
            next_cursor,
        )
        .into_response()?)
    }
}

#[async_trait]
impl Tool for ListChannelMembersTool {
    fn description(&self) -> &str {
        "List channel members with details"
    }

    async fn execute(&self, params: Value) -> McpResult<Value> {
        // Parse parameters
        let params: ListChannelMembersParams = parse_params(params)?;

        // Resolve channel ID if name is provided
        let channel_id = resolve_channel_id(&params.channel, &self.cache, None).await?;

        // Get channel members
        let (member_ids, _) = self
            .slack_client
            .channels
            .get_channel_members(
                &channel_id,
                params.limit,
                None, // Always start from beginning
            )
            .await
            .mcp_context("Failed to get channel members")?;

        // Get user details from cache
        let users = self
            .cache
            .get_users()
            .await
            .mcp_context("Failed to get users")?;

        // Match member IDs with user details
        let members: Vec<Value> = member_ids
            .iter()
            .filter_map(|id| {
                users.iter().find(|u| &u.id == id).map(|u| {
                    json!({
                        "id": u.id,
                        "name": u.name,
                        "real_name": u.real_name(),
                        "is_bot": u.is_bot,
                        "is_admin": u.is_admin,
                    })
                })
            })
            .collect();

        Ok(ToolResponse::data(json!({
            "members": members,
            "count": members.len(),
        }))
        .into_response()?)
    }
}
