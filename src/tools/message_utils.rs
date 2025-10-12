use crate::cache::SqliteCache;
use crate::slack::types::{SlackMessage, SlackUser};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};
use std::sync::Arc;

/// Remove fields with empty string values from JSON object
fn remove_empty_strings(value: &mut Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.retain(|_, v| {
            if let Some(s) = v.as_str() {
                !s.is_empty()
            } else {
                true
            }
        });
    }
}

/// Convert Slack timestamp to ISO 8601 format
/// Slack timestamps are Unix timestamps with microseconds (e.g., "1234567890.123456")
fn slack_ts_to_iso8601(ts: &str) -> Option<String> {
    // Parse the timestamp as a float and convert to seconds and nanoseconds
    ts.parse::<f64>().ok().and_then(|timestamp| {
        let seconds = timestamp as i64;
        let nanos = ((timestamp - seconds as f64) * 1_000_000_000.0) as u32;

        Utc.timestamp_opt(seconds, nanos)
            .single()
            .map(|dt: DateTime<Utc>| dt.to_rfc3339())
    })
}

/// Get display name from a user, checking for empty strings
pub fn get_user_display_name(user: &SlackUser) -> &str {
    if let Some(profile) = &user.profile {
        // Check display_name first, then real_name, fallback to username
        if let Some(display_name) = &profile.display_name
            && !display_name.trim().is_empty()
        {
            return display_name;
        }
        if let Some(real_name) = &profile.real_name
            && !real_name.trim().is_empty()
        {
            return real_name;
        }
    }
    &user.name
}

/// Format a message with user name resolution
pub async fn format_message(
    msg: SlackMessage,
    cache: &Arc<SqliteCache>,
    include_thread_info: bool,
) -> Value {
    let mut result = json!({
        "ts": msg.ts.clone(),
        "text": msg.text,
    });

    // Add ISO 8601 formatted datetime
    if let Some(iso_time) = slack_ts_to_iso8601(&msg.ts) {
        result["datetime"] = json!(iso_time);
    }

    // Add user_id with name resolution if present
    if let Some(user_id) = msg.user {
        result["user_id"] = json!(user_id);

        // Try to get user name from cache
        if let Ok(Some(user)) = cache.get_user_by_id(&user_id).await {
            result["user_name"] = json!(get_user_display_name(&user));
        }
    }

    // Add channel information if available (from search.messages)
    if let Some(channel) = &msg.channel {
        result["channel_id"] = json!(channel.id);
        result["channel_name"] = json!(channel.name);
    }

    // Add thread information if requested
    if include_thread_info && let Some(thread_ts) = &msg.thread_ts {
        result["thread_ts"] = json!(thread_ts);

        // Add ISO 8601 formatted thread datetime
        if let Some(iso_time) = slack_ts_to_iso8601(thread_ts) {
            result["thread_datetime"] = json!(iso_time);
        }

        // Check if this is a thread parent or reply
        if thread_ts == &msg.ts {
            // This is a thread parent
            result["is_thread_parent"] = json!(true);
            if let Some(reply_count) = msg.reply_count
                && reply_count > 0
            {
                result["reply_count"] = json!(reply_count);
            }
            if let Some(latest_reply) = msg.latest_reply {
                result["latest_reply"] = json!(latest_reply.clone());

                // Add ISO 8601 formatted latest reply datetime
                if let Some(iso_time) = slack_ts_to_iso8601(&latest_reply) {
                    result["latest_reply_datetime"] = json!(iso_time);
                }
            }
        } else {
            // This is a thread reply - no parent_user info needed
            result["is_thread_reply"] = json!(true);
        }
    }

    remove_empty_strings(&mut result);
    result
}

/// Format thread messages with parent info only once
pub async fn format_thread_messages(
    messages: Vec<SlackMessage>,
    cache: &Arc<SqliteCache>,
) -> Value {
    if messages.is_empty() {
        return json!({
            "messages": []
        });
    }

    let mut result = json!({});
    let mut formatted_messages = Vec::new();

    // Check if first message is the parent
    let first_msg = &messages[0];
    if let Some(thread_ts) = &first_msg.thread_ts
        && thread_ts == &first_msg.ts
    {
        // First message is the parent - extract parent info
        let mut parent_info = json!({
            "parent_ts": first_msg.ts.clone(),
            "parent_text": first_msg.text.clone(),
        });

        // Add ISO 8601 formatted parent datetime
        if let Some(iso_time) = slack_ts_to_iso8601(&first_msg.ts) {
            parent_info["parent_datetime"] = json!(iso_time);
        }

        if let Some(user_id) = &first_msg.user {
            parent_info["parent_user_id"] = json!(user_id);
            if let Ok(Some(user)) = cache.get_user_by_id(user_id).await {
                parent_info["parent_user_name"] = json!(get_user_display_name(&user));
            }
        }

        result["thread_info"] = parent_info;
    }

    // Format all messages without parent_user duplication
    for msg in messages {
        formatted_messages.push(format_message(msg, cache, false).await);
    }

    result["messages"] = json!(formatted_messages);

    // Remove empty strings from thread_info
    if let Some(thread_info) = result.get_mut("thread_info") {
        remove_empty_strings(thread_info);
    }

    // Remove empty strings from each message
    if let Some(messages) = result.get_mut("messages")
        && let Some(messages_array) = messages.as_array_mut()
    {
        for msg in messages_array {
            remove_empty_strings(msg);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slack::types::{MessageChannel, SlackUserProfile};

    async fn setup_cache() -> Arc<SqliteCache> {
        Arc::new(
            SqliteCache::new(":memory:")
                .await
                .expect("Failed to create test cache"),
        )
    }

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

    fn create_test_message(ts: &str, text: &str, user_id: Option<&str>) -> SlackMessage {
        SlackMessage {
            ts: ts.to_string(),
            user: user_id.map(|s| s.to_string()),
            text: text.to_string(),
            channel: None,
            thread_ts: None,
            reply_count: None,
            reply_users: None,
            reply_users_count: None,
            latest_reply: None,
            parent_user_id: None,
            reactions: None,
            subtype: None,
            edited: None,
            blocks: None,
            attachments: None,
        }
    }

    // Tests for slack_ts_to_iso8601

    #[test]
    fn test_slack_ts_to_iso8601_valid() {
        let result = slack_ts_to_iso8601("1609459200.000000");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "2021-01-01T00:00:00+00:00");
    }

    #[test]
    fn test_slack_ts_to_iso8601_with_microseconds() {
        let result = slack_ts_to_iso8601("1609459200.123456");
        assert!(result.is_some());
        let iso = result.unwrap();
        assert!(iso.starts_with("2021-01-01T00:00:00.123"));
    }

    #[test]
    fn test_slack_ts_to_iso8601_invalid() {
        let result = slack_ts_to_iso8601("invalid");
        assert!(result.is_none());
    }

    #[test]
    fn test_slack_ts_to_iso8601_empty() {
        let result = slack_ts_to_iso8601("");
        assert!(result.is_none());
    }

    // Tests for get_user_display_name

    #[test]
    fn test_get_user_display_name_prefers_display_name() {
        let user = create_test_user("U123", "alice", Some("Alice Wonder"));
        assert_eq!(get_user_display_name(&user), "Alice Wonder");
    }

    #[test]
    fn test_get_user_display_name_falls_back_to_real_name() {
        let user = create_test_user("U123", "alice", None);
        assert_eq!(get_user_display_name(&user), "alice");
    }

    #[test]
    fn test_get_user_display_name_falls_back_to_username() {
        let mut user = create_test_user("U123", "alice", None);
        if let Some(profile) = &mut user.profile {
            profile.real_name = None;
        }
        assert_eq!(get_user_display_name(&user), "alice");
    }

    #[test]
    fn test_get_user_display_name_ignores_empty_display_name() {
        let user = create_test_user("U123", "alice", Some(""));
        assert_eq!(get_user_display_name(&user), "alice");
    }

    #[test]
    fn test_get_user_display_name_ignores_whitespace_only() {
        let user = create_test_user("U123", "alice", Some("   "));
        assert_eq!(get_user_display_name(&user), "alice");
    }

    // Tests for remove_empty_strings

    #[test]
    fn test_remove_empty_strings() {
        let mut value = json!({
            "field1": "value",
            "field2": "",
            "field3": "another value",
            "field4": "",
        });
        remove_empty_strings(&mut value);

        assert!(value["field1"].is_string());
        assert!(value["field2"].is_null());
        assert!(value["field3"].is_string());
        assert!(value["field4"].is_null());
    }

    #[test]
    fn test_remove_empty_strings_preserves_non_empty() {
        let mut value = json!({
            "text": "Hello World",
            "number": 42,
            "boolean": true,
        });
        remove_empty_strings(&mut value);

        assert_eq!(value["text"], "Hello World");
        assert_eq!(value["number"], 42);
        assert_eq!(value["boolean"], true);
    }

    // Tests for format_message

    #[tokio::test]
    async fn test_format_message_basic() {
        let cache = setup_cache().await;
        let msg = create_test_message("1609459200.000000", "Hello World", None);

        let result = format_message(msg, &cache, false).await;

        assert_eq!(result["ts"], "1609459200.000000");
        assert_eq!(result["text"], "Hello World");
        assert_eq!(result["datetime"], "2021-01-01T00:00:00+00:00");
    }

    #[tokio::test]
    async fn test_format_message_with_user() {
        let cache = setup_cache().await;

        // Add user to cache
        let user = create_test_user("U123", "alice", Some("Alice"));
        cache.save_users(vec![user]).await.unwrap();

        let msg = create_test_message("1609459200.000000", "Hello", Some("U123"));

        let result = format_message(msg, &cache, false).await;

        assert_eq!(result["user_id"], "U123");
        assert_eq!(result["user_name"], "Alice");
    }

    #[tokio::test]
    async fn test_format_message_user_not_in_cache() {
        let cache = setup_cache().await;

        let msg = create_test_message("1609459200.000000", "Hello", Some("U999"));

        let result = format_message(msg, &cache, false).await;

        assert_eq!(result["user_id"], "U999");
        assert!(result["user_name"].is_null());
    }

    #[tokio::test]
    async fn test_format_message_with_channel() {
        let cache = setup_cache().await;

        let mut msg = create_test_message("1609459200.000000", "Hello", None);
        msg.channel = Some(MessageChannel {
            id: "C123".to_string(),
            name: "general".to_string(),
        });

        let result = format_message(msg, &cache, false).await;

        assert_eq!(result["channel_id"], "C123");
        assert_eq!(result["channel_name"], "general");
    }

    #[tokio::test]
    async fn test_format_message_thread_parent() {
        let cache = setup_cache().await;

        let mut msg = create_test_message("1609459200.000000", "Thread start", Some("U123"));
        msg.thread_ts = Some("1609459200.000000".to_string());
        msg.reply_count = Some(5);
        msg.latest_reply = Some("1609459300.000000".to_string());

        let result = format_message(msg, &cache, true).await;

        assert_eq!(result["is_thread_parent"], true);
        assert_eq!(result["thread_ts"], "1609459200.000000");
        assert_eq!(result["reply_count"], 5);
        assert_eq!(result["latest_reply"], "1609459300.000000");
    }

    #[tokio::test]
    async fn test_format_message_thread_reply() {
        let cache = setup_cache().await;

        let mut msg = create_test_message("1609459250.000000", "Thread reply", Some("U456"));
        msg.thread_ts = Some("1609459200.000000".to_string());

        let result = format_message(msg, &cache, true).await;

        assert_eq!(result["is_thread_reply"], true);
        assert_eq!(result["thread_ts"], "1609459200.000000");
        assert!(result["is_thread_parent"].is_null());
    }

    #[tokio::test]
    async fn test_format_message_without_thread_info() {
        let cache = setup_cache().await;

        let mut msg = create_test_message("1609459200.000000", "Message", None);
        msg.thread_ts = Some("1609459200.000000".to_string());

        let result = format_message(msg, &cache, false).await;

        // Thread info should not be included
        assert!(result["thread_ts"].is_null());
        assert!(result["is_thread_parent"].is_null());
    }

    // Tests for format_thread_messages

    #[tokio::test]
    async fn test_format_thread_messages_empty() {
        let cache = setup_cache().await;

        let result = format_thread_messages(vec![], &cache).await;

        assert!(result["messages"].is_array());
        assert_eq!(result["messages"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_format_thread_messages_with_parent() {
        let cache = setup_cache().await;

        // Add user to cache
        let user = create_test_user("U123", "alice", Some("Alice"));
        cache.save_users(vec![user]).await.unwrap();

        let mut parent = create_test_message("1609459200.000000", "Parent message", Some("U123"));
        parent.thread_ts = Some("1609459200.000000".to_string());

        let mut reply = create_test_message("1609459250.000000", "Reply", Some("U123"));
        reply.thread_ts = Some("1609459200.000000".to_string());

        let result = format_thread_messages(vec![parent, reply], &cache).await;

        // Should have thread_info
        assert!(!result["thread_info"].is_null());
        assert_eq!(result["thread_info"]["parent_ts"], "1609459200.000000");
        assert_eq!(result["thread_info"]["parent_text"], "Parent message");
        assert_eq!(result["thread_info"]["parent_user_id"], "U123");
        assert_eq!(result["thread_info"]["parent_user_name"], "Alice");

        // Should have 2 messages
        assert_eq!(result["messages"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_format_thread_messages_without_parent() {
        let cache = setup_cache().await;

        let mut msg1 = create_test_message("1609459250.000000", "Reply 1", Some("U123"));
        msg1.thread_ts = Some("1609459200.000000".to_string());

        let mut msg2 = create_test_message("1609459300.000000", "Reply 2", Some("U456"));
        msg2.thread_ts = Some("1609459200.000000".to_string());

        let result = format_thread_messages(vec![msg1, msg2], &cache).await;

        // Should not have thread_info (parent not included)
        assert!(result["thread_info"].is_null());

        // Should have 2 messages
        assert_eq!(result["messages"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_format_thread_messages_removes_empty_strings() {
        let cache = setup_cache().await;

        let msg = create_test_message("1609459200.000000", "Message", None);

        let result = format_thread_messages(vec![msg], &cache).await;

        // Check that empty user fields are not included
        let first_msg = &result["messages"][0];
        assert!(first_msg["user_id"].is_null());
        assert!(first_msg["user_name"].is_null());
    }
}
