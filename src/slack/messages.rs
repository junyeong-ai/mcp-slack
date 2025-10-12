use anyhow::Result;
use serde_json::{Value, json};
use std::sync::Arc;

use super::core::SlackCore;
use crate::slack::SlackMessage;

pub struct SlackMessageClient {
    core: Arc<SlackCore>,
}

impl SlackMessageClient {
    pub fn new(core: Arc<SlackCore>) -> Self {
        Self { core }
    }

    /// Send a message to a channel
    pub async fn post_message(
        &self,
        channel: &str,
        text: Option<&str>,
        blocks: Option<&Vec<Value>>,
        thread_ts: Option<&str>,
        reply_broadcast: bool,
    ) -> Result<String> {
        let mut params = json!({
            "channel": channel,
            "reply_broadcast": reply_broadcast,
        });

        if let Some(text) = text {
            params["text"] = json!(text);
        }

        if let Some(blocks) = blocks {
            params["blocks"] = json!(blocks);
        }

        if let Some(ts) = thread_ts {
            params["thread_ts"] = json!(ts);
        }

        let response = self
            .core
            .api_call("chat.postMessage", params, None, false)
            .await?;

        let timestamp = response["ts"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing timestamp in response"))?;

        Ok(timestamp.to_string())
    }

    /// Get channel messages
    pub async fn get_channel_messages(
        &self,
        channel: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<(Vec<SlackMessage>, Option<String>)> {
        let mut params = json!({
            "channel": channel,
            "limit": limit,
        });

        if let Some(cursor) = cursor {
            params["cursor"] = json!(cursor);
        }

        let response = self
            .core
            .api_call("conversations.history", params, None, false)
            .await?;

        let messages: Vec<SlackMessage> = response["messages"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|m| serde_json::from_value(m.clone()).ok())
            .collect();

        let next_cursor = response["response_metadata"]["next_cursor"]
            .as_str()
            .filter(|c| !c.is_empty())
            .map(|c| c.to_string());

        Ok((messages, next_cursor))
    }

    /// Get thread replies
    pub async fn get_thread_replies(
        &self,
        channel: &str,
        thread_ts: &str,
        limit: usize,
    ) -> Result<(Vec<SlackMessage>, bool)> {
        let params = json!({
            "channel": channel,
            "ts": thread_ts,
            "limit": limit,
        });

        let response = self
            .core
            .api_call("conversations.replies", params, None, false)
            .await?;

        let messages: Vec<SlackMessage> = response["messages"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|m| serde_json::from_value(m.clone()).ok())
            .collect();

        let has_more = response["has_more"].as_bool().unwrap_or(false);

        Ok((messages, has_more))
    }

    /// Search messages
    pub async fn search_messages(
        &self,
        query: &str,
        channel: Option<&str>,
        from_user: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SlackMessage>> {
        let mut search_query = query.to_string();

        if let Some(channel) = channel {
            search_query.push_str(&format!(" in:{}", channel));
        }

        if let Some(user) = from_user {
            search_query.push_str(&format!(" from:{}", user));
        }

        let params = json!({
            "query": search_query,
            "count": limit,
        });

        let response = self
            .core
            .api_call("search.messages", params, None, true)
            .await?;

        let messages: Vec<SlackMessage> = response["messages"]["matches"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|m| serde_json::from_value(m.clone()).ok())
            .collect();

        Ok(messages)
    }
}
