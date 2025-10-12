use crate::cache::SqliteCache;
use crate::slack::types::{SlackMessage, SlackUser};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};
use std::sync::Arc;

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

    // Add blocks if present (for rich content from bots)
    if let Some(blocks) = &msg.blocks
        && !blocks.is_empty()
    {
        result["blocks"] = json!(blocks);
    }

    // Add attachments if present
    if let Some(attachments) = &msg.attachments
        && !attachments.is_empty()
    {
        result["attachments"] = json!(attachments);
    }

    // Add user_id with name resolution if present
    if let Some(user_id) = msg.user {
        result["user_id"] = json!(user_id);

        // Try to get user name from cache
        if let Ok(Some(user)) = cache.get_user_by_id(&user_id).await {
            result["user_name"] = json!(get_user_display_name(&user));
        }
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

        // Add blocks/attachments if present for parent
        if let Some(blocks) = &first_msg.blocks
            && !blocks.is_empty()
        {
            parent_info["parent_blocks"] = json!(blocks);
        }
        if let Some(attachments) = &first_msg.attachments
            && !attachments.is_empty()
        {
            parent_info["parent_attachments"] = json!(attachments);
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
    result
}
