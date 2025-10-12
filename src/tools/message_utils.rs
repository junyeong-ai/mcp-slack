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
    if let Some(messages) = result.get_mut("messages") {
        if let Some(messages_array) = messages.as_array_mut() {
            for msg in messages_array {
                remove_empty_strings(msg);
            }
        }
    }

    result
}
