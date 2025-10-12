use crate::cache::SqliteCache;
use crate::error::{IntoMcpError, McpError, McpResult};
use crate::slack::SlackClient;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;

const CHANNEL_SEARCH_LIMIT: usize = 1;

/// Parse JSON value into a typed parameter struct
pub fn parse_params<T: DeserializeOwned>(params: Value) -> McpResult<T> {
    serde_json::from_value(params)
        .map_err(|e| McpError::InvalidParameter(format!("Invalid parameters: {}", e)))
}

/// Validate that at least one of the required fields is present
pub fn validate_required_one_of<T, U>(
    value: &Option<T>,
    other: &Option<U>,
    field_names: &str,
) -> McpResult<()> {
    if value.is_none() && other.is_none() {
        return Err(McpError::InvalidParameter(format!(
            "Either {} must be provided",
            field_names
        )));
    }
    Ok(())
}

/// Resolve channel identifier to channel ID
/// Supports:
/// - Channel IDs (C..., G..., D...)
/// - Channel names (without #)
/// - #channel-name format
/// - @username format (opens DM)
pub async fn resolve_channel_id(
    identifier: &str,
    cache: &Arc<SqliteCache>,
    slack_client: Option<&Arc<SlackClient>>,
) -> McpResult<String> {
    // Already a channel ID (starts with C, G, or D)
    // Return immediately without cache lookup - let the API call handle access validation
    if identifier.starts_with('C') || identifier.starts_with('G') || identifier.starts_with('D') {
        return Ok(identifier.to_string());
    }

    // Handle #channel-name format
    let channel_name = if let Some(stripped) = identifier.strip_prefix('#') {
        stripped
    } else if identifier.starts_with('@') && slack_client.is_some() {
        // Handle @username format - resolve to DM
        let username = &identifier[1..];
        let users = cache.get_users().await.mcp_context("Failed to get users")?;

        let user_id = users
            .iter()
            .find(|u| u.name == username || u.display_name() == Some(username))
            .map(|u| u.id.clone())
            .ok_or_else(|| {
                McpError::InvalidParameter(format!("User '{}' not found", identifier))
            })?;

        // Open DM channel with user
        let client = slack_client.ok_or_else(|| {
            McpError::InvalidParameter("Slack client required for opening DM channels".to_string())
        })?;
        return client
            .users
            .open_conversation(&user_id)
            .await
            .mcp_context("Failed to open DM");
    } else {
        identifier
    };

    // Search for channel by name in cache
    let channels = cache
        .search_channels(channel_name, CHANNEL_SEARCH_LIMIT)
        .await
        .mcp_context("Failed to search channels")?;

    if !channels.is_empty() && channels[0].name == channel_name {
        Ok(channels[0].id.clone())
    } else {
        Err(McpError::InvalidParameter(format!(
            "Channel '{}' not found",
            identifier
        )))
    }
}
