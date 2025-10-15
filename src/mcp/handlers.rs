use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

use crate::cache::SqliteCache;
use crate::config::Config;
use crate::error::McpError;
use crate::slack::SlackClient;
use crate::tools::{Tool, cache as cache_tools, messages, search};

use super::types::{CallToolResult, Property, Tool as McpTool, ToolContent, ToolInputSchema};

pub struct RequestHandler {
    tools: HashMap<String, Box<dyn Tool + Send + Sync>>,
}

macro_rules! register_tool {
    ($tools:expr, $name:expr, $tool:expr) => {
        $tools.insert($name.to_string(), Box::new($tool));
    };
}

impl RequestHandler {
    pub async fn new(
        cache: Arc<SqliteCache>,
        slack_client: Arc<SlackClient>,
        _config: Config,
    ) -> anyhow::Result<Self> {
        let mut tools: HashMap<String, Box<dyn Tool + Send + Sync>> = HashMap::new();

        // Register search tools
        register_tool!(
            tools,
            "search_users",
            search::SearchUsersTool::new(cache.clone())
        );
        register_tool!(
            tools,
            "search_channels",
            search::SearchChannelsTool::new(cache.clone())
        );
        register_tool!(
            tools,
            "search_messages",
            search::SearchMessagesTool::new(slack_client.clone(), cache.clone())
        );

        // Register message tools
        register_tool!(
            tools,
            "send_message",
            messages::SendMessageTool::new(slack_client.clone(), cache.clone())
        );
        register_tool!(
            tools,
            "read_thread",
            messages::ReadThreadTool::new(slack_client.clone(), cache.clone())
        );
        register_tool!(
            tools,
            "list_channel_members",
            messages::ListChannelMembersTool::new(slack_client.clone(), cache.clone())
        );
        register_tool!(
            tools,
            "get_channel_messages",
            messages::GetChannelMessagesTool::new(slack_client.clone(), cache.clone())
        );

        // Register cache tool
        register_tool!(
            tools,
            "refresh_cache",
            cache_tools::RefreshCacheTool::new(slack_client.clone(), cache.clone())
        );

        // Check Slack token status
        let has_bot_token = _config.slack.bot_token.is_some();
        let has_user_token = _config.slack.user_token.is_some();

        if !has_bot_token && !has_user_token {
            warn!(
                "No Slack tokens configured! Set SLACK_BOT_TOKEN or SLACK_USER_TOKEN environment variable, or create config file."
            );
        }

        // Initialize cache if empty or stale
        if has_bot_token || has_user_token {
            let (user_count, channel_count) = cache.get_counts().unwrap_or((0, 0));
            // Use the minimum TTL of users and channels
            let cache_ttl_hours = _config
                .cache
                .ttl_users_hours
                .min(_config.cache.ttl_channels_hours) as i64;
            let is_stale = cache.is_cache_stale(Some(cache_ttl_hours)).unwrap_or(true);

            if (user_count == 0 && channel_count == 0) || is_stale {
                // Cache is empty or stale, perform initial/refresh load
                tokio::spawn({
                    let slack_client = slack_client.clone();
                    let cache = cache.clone();
                    async move {
                        // Fetch users
                        if let Ok(users) = slack_client.users.fetch_all_users().await {
                            let _ = cache.save_users(users).await;
                        }

                        // Fetch channels
                        if let Ok(channels) = slack_client.channels.fetch_all_channels().await {
                            let _ = cache.save_channels(channels).await;
                        }
                    }
                });
            }
        }

        Ok(Self { tools })
    }

    pub async fn list_tools(&self) -> Vec<McpTool> {
        let mut tool_list = Vec::new();

        for (name, tool) in &self.tools {
            tool_list.push(self.tool_to_mcp_tool(name, tool.as_ref()));
        }

        tool_list
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<CallToolResult, McpError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| McpError::NotFound(format!("Tool not found: {}", name)))?;

        let result = tool.execute(arguments).await?;

        // Convert result to tool content
        let content = if let Some(text) = result.as_str() {
            vec![ToolContent::Text {
                text: text.to_string(),
            }]
        } else {
            vec![ToolContent::Text {
                text: serde_json::to_string_pretty(&result)?,
            }]
        };

        Ok(CallToolResult { content })
    }

    // Helper functions for creating tool schemas
    fn create_string_prop(description: &str, _required: bool) -> Property {
        Property {
            property_type: "string".to_string(),
            description: Some(description.to_string()),
            default: None,
            enum_values: None,
        }
    }

    fn create_number_prop(description: &str, default: i32) -> Property {
        Property {
            property_type: "number".to_string(),
            description: Some(description.to_string()),
            default: Some(Value::Number(default.into())),
            enum_values: None,
        }
    }

    fn create_enum_prop(description: &str, default: &str, options: Vec<&str>) -> Property {
        Property {
            property_type: "string".to_string(),
            description: Some(description.to_string()),
            default: Some(Value::String(default.to_string())),
            enum_values: Some(
                options
                    .into_iter()
                    .map(|s| Value::String(s.to_string()))
                    .collect(),
            ),
        }
    }

    fn tool_to_mcp_tool(&self, name: &str, tool: &(dyn Tool + Send + Sync)) -> McpTool {
        // Create input schema based on tool name
        let (properties, required) = match name {
            "search_users" => {
                let mut props = HashMap::new();
                props.insert(
                    "query".to_string(),
                    Self::create_string_prop("Search query for user name or email", true),
                );
                props.insert(
                    "limit".to_string(),
                    Self::create_number_prop("Maximum number of results (default: 10)", 10),
                );
                (props, vec!["query".to_string()])
            }
            "search_channels" => {
                let mut props = HashMap::new();
                props.insert(
                    "query".to_string(),
                    Self::create_string_prop("Search query for channel name", true),
                );
                props.insert(
                    "limit".to_string(),
                    Self::create_number_prop("Maximum number of results (default: 10)", 10),
                );
                (props, vec!["query".to_string()])
            }
            "send_message" => {
                let mut props = HashMap::new();
                props.insert(
                    "channel".to_string(),
                    Self::create_string_prop("Channel ID or user ID to send message to", true),
                );
                props.insert(
                    "text".to_string(),
                    Self::create_string_prop("Message text to send", true),
                );
                props.insert(
                    "thread_ts".to_string(),
                    Self::create_string_prop("Thread timestamp to reply to (optional)", false),
                );
                (props, vec!["channel".to_string(), "text".to_string()])
            }
            "list_channel_members" => {
                let mut props = HashMap::new();
                props.insert(
                    "channel".to_string(),
                    Self::create_string_prop("Channel ID to list members from", true),
                );
                (props, vec!["channel".to_string()])
            }
            "get_channel_messages" => {
                let mut props = HashMap::new();
                props.insert(
                    "channel".to_string(),
                    Self::create_string_prop(
                        "Channel ID (C..., G..., D...) or exact channel name",
                        true,
                    ),
                );
                props.insert(
                    "limit".to_string(),
                    Self::create_number_prop("Maximum number of messages (default: 100)", 100),
                );
                props.insert(
                    "cursor".to_string(),
                    Self::create_string_prop("Pagination cursor (optional)", false),
                );
                (props, vec!["channel".to_string()])
            }
            "refresh_cache" => {
                let mut props = HashMap::new();
                props.insert(
                    "type".to_string(),
                    Self::create_enum_prop(
                        "Type of data to refresh",
                        "all",
                        vec!["users", "channels", "all"],
                    ),
                );
                (props, vec![])
            }
            "search_messages" => {
                let mut props = HashMap::new();
                props.insert(
                    "query".to_string(),
                    Self::create_string_prop("Search query for messages", true),
                );
                props.insert(
                    "channel".to_string(),
                    Self::create_string_prop("Channel to search in (optional)", false),
                );
                props.insert(
                    "from_user".to_string(),
                    Self::create_string_prop("User to search messages from (optional)", false),
                );
                props.insert(
                    "limit".to_string(),
                    Self::create_number_prop("Maximum number of results (default: 10)", 10),
                );
                (props, vec!["query".to_string()])
            }
            "read_thread" => {
                let mut props = HashMap::new();
                props.insert(
                    "channel".to_string(),
                    Self::create_string_prop("Channel ID containing the thread", true),
                );
                props.insert(
                    "thread_ts".to_string(),
                    Self::create_string_prop("Thread timestamp to read", true),
                );
                props.insert(
                    "limit".to_string(),
                    Self::create_number_prop("Maximum number of messages (default: 100)", 100),
                );
                (props, vec!["channel".to_string(), "thread_ts".to_string()])
            }
            _ => (HashMap::new(), vec![]),
        };

        McpTool {
            name: name.to_string(),
            description: tool.description().to_string(),
            input_schema: ToolInputSchema {
                schema_type: "object".to_string(),
                properties,
                required,
            },
        }
    }
}
