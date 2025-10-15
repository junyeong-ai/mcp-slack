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
        let users = cache.get_users().mcp_context("Failed to get users")?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::SqliteCache;
    use crate::slack::types::{SlackChannel, SlackUser, SlackUserProfile};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct TestParams {
        name: String,
        count: i32,
    }

    async fn setup_cache() -> Arc<SqliteCache> {
        Arc::new(
            SqliteCache::new(":memory:")
                .await
                .expect("Failed to create test cache"),
        )
    }

    #[allow(dead_code)]
    fn create_test_user(id: &str, name: &str, display_name: Option<&str>) -> SlackUser {
        SlackUser {
            id: id.to_string(),
            name: name.to_string(),
            is_bot: false,
            is_admin: false,
            deleted: false,
            profile: Some(SlackUserProfile {
                real_name: Some(name.to_string()),
                display_name: display_name.map(|s| s.to_string()),
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

    #[test]
    fn test_parse_params_valid() {
        let params = json!({"name": "test", "count": 42});
        let result: McpResult<TestParams> = parse_params(params);

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.count, 42);
    }

    #[test]
    fn test_parse_params_invalid_type() {
        let params = json!({"name": "test", "count": "not_a_number"});
        let result: McpResult<TestParams> = parse_params(params);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, McpError::InvalidParameter(_)));
    }

    #[test]
    fn test_parse_params_missing_field() {
        let params = json!({"name": "test"});
        let result: McpResult<TestParams> = parse_params(params);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), McpError::InvalidParameter(_)));
    }

    #[test]
    fn test_validate_required_one_of_both_present() {
        let value1 = Some("value1");
        let value2 = Some("value2");
        let result = validate_required_one_of(&value1, &value2, "field1 or field2");

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_required_one_of_first_present() {
        let value1 = Some("value1");
        let value2: Option<String> = None;
        let result = validate_required_one_of(&value1, &value2, "field1 or field2");

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_required_one_of_second_present() {
        let value1: Option<String> = None;
        let value2 = Some("value2");
        let result = validate_required_one_of(&value1, &value2, "field1 or field2");

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_required_one_of_both_none() {
        let value1: Option<String> = None;
        let value2: Option<String> = None;
        let result = validate_required_one_of(&value1, &value2, "field1 or field2");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, McpError::InvalidParameter(_)));
        assert!(err.to_string().contains("field1 or field2"));
    }

    #[tokio::test]
    async fn test_resolve_channel_id_with_channel_id() {
        let cache = setup_cache().await;

        // Channel IDs starting with C, G, or D should be returned as-is
        let result = resolve_channel_id("C123456", &cache, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "C123456");

        let result = resolve_channel_id("G789ABC", &cache, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "G789ABC");

        let result = resolve_channel_id("D456DEF", &cache, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "D456DEF");
    }

    #[tokio::test]
    async fn test_resolve_channel_id_with_channel_name() {
        let cache = setup_cache().await;

        // Save test channel
        let channels = vec![create_test_channel("C123", "general")];
        cache.save_channels(channels).await.unwrap();

        let result = resolve_channel_id("general", &cache, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "C123");
    }

    #[tokio::test]
    async fn test_resolve_channel_id_with_hash_prefix() {
        let cache = setup_cache().await;

        let channels = vec![create_test_channel("C456", "random")];
        cache.save_channels(channels).await.unwrap();

        let result = resolve_channel_id("#random", &cache, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "C456");
    }

    #[tokio::test]
    async fn test_resolve_channel_id_not_found() {
        let cache = setup_cache().await;

        let result = resolve_channel_id("nonexistent", &cache, None).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, McpError::InvalidParameter(_)));
        assert!(err.to_string().contains("Channel 'nonexistent' not found"));
    }

    #[tokio::test]
    async fn test_resolve_channel_id_hash_not_found() {
        let cache = setup_cache().await;

        let result = resolve_channel_id("#missing", &cache, None).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Channel '#missing' not found")
        );
    }
}
