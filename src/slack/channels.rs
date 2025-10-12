use anyhow::Result;
use serde_json::json;
use std::sync::Arc;

use super::core::SlackCore;
use crate::slack::SlackChannel;

const SLACK_API_LIMIT: u32 = 200;

pub struct SlackChannelClient {
    pub(crate) core: Arc<SlackCore>,
}

impl SlackChannelClient {
    pub fn new(core: Arc<SlackCore>) -> Self {
        Self { core }
    }

    /// Fetch all channels from the workspace
    /// Uses user token when available to get private channels the user has access to
    pub async fn fetch_all_channels(&self) -> Result<Vec<SlackChannel>> {
        let mut all_channels = Vec::new();

        self.fetch_all_channels_streaming(|channels| {
            all_channels.extend(channels);
            Ok(())
        })
        .await?;

        Ok(all_channels)
    }

    /// Stream fetch channels with callback for immediate processing of each page
    pub async fn fetch_all_channels_streaming<F>(&self, mut callback: F) -> Result<usize>
    where
        F: FnMut(Vec<SlackChannel>) -> Result<()>,
    {
        let mut total_fetched = 0;
        let mut cursor: Option<String> = None;
        let limit = SLACK_API_LIMIT;

        loop {
            let mut params = json!({
                "limit": limit,
                "types": "public_channel,private_channel",
                "exclude_archived": false,
            });

            if let Some(cursor_val) = &cursor {
                params["cursor"] = json!(cursor_val);
            }

            // Use user token preference to get private channels
            let response = self
                .core
                .api_call("conversations.list", params, None, true)
                .await?;

            // Parse channels from response
            let mut page_channels = Vec::new();
            if let Some(channels) = response["channels"].as_array() {
                for channel in channels {
                    match serde_json::from_value::<SlackChannel>(channel.clone()) {
                        Ok(channel_obj) => {
                            page_channels.push(channel_obj);
                        }
                        Err(_) => {
                            // Skip malformed channel
                        }
                    }
                }
            }

            // Process this page immediately via callback
            if !page_channels.is_empty() {
                let page_count = page_channels.len();
                callback(page_channels)?;
                total_fetched += page_count;
            }

            // Check for pagination
            cursor = response["response_metadata"]["next_cursor"]
                .as_str()
                .filter(|c| !c.is_empty())
                .map(|c| c.to_string());

            if cursor.is_none() {
                break;
            }
        }

        Ok(total_fetched)
    }

    /// Get channel members
    pub async fn get_channel_members(
        &self,
        channel: &str,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<(Vec<String>, Option<String>)> {
        let mut params = json!({
            "channel": channel,
            "limit": limit,
        });

        if let Some(cursor) = cursor {
            params["cursor"] = json!(cursor);
        }

        let response = self
            .core
            .api_call("conversations.members", params, None, true)
            .await?;

        let members: Vec<String> = response["members"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|m| m.as_str().map(|s| s.to_string()))
            .collect();

        let next_cursor = response["response_metadata"]["next_cursor"]
            .as_str()
            .filter(|c| !c.is_empty())
            .map(|c| c.to_string());

        Ok((members, next_cursor))
    }
}
