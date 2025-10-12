use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::RwLock;
use tracing::{error, warn};

use crate::cache::SqliteCache;
use crate::config::Config;
use crate::slack::SlackClient;

use super::handlers::RequestHandler;
use super::types::*;

pub struct McpServer {
    _config: Config,
    handler: Arc<RequestHandler>,
    initialized: Arc<RwLock<bool>>,
}

impl McpServer {
    pub async fn new(
        config: Config,
        cache: Arc<SqliteCache>,
        slack_client: Arc<SlackClient>,
    ) -> Result<Self> {
        // Create handler with tools
        let handler =
            RequestHandler::new(cache.clone(), slack_client.clone(), config.clone()).await?;

        Ok(Self {
            _config: config,
            handler: Arc::new(handler),
            initialized: Arc::new(RwLock::new(false)),
        })
    }

    pub async fn run(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut stdout = stdout;

        let mut buffer = String::new();
        let mut empty_reads = 0;

        loop {
            buffer.clear();

            // Read a line from stdin
            match reader.read_line(&mut buffer).await {
                Ok(0) => {
                    empty_reads += 1;

                    // Give it a few chances before exiting
                    if empty_reads > 3 {
                        break;
                    }
                    // Small delay before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    continue;
                }
                Ok(_) => {
                    empty_reads = 0; // Reset counter on successful read
                    let trimmed = buffer.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // Process the request
                    match self.process_request(trimmed).await {
                        Ok(Some(response)) => {
                            let response_str = serde_json::to_string(&response)?;

                            stdout.write_all(response_str.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                        Ok(None) => {
                            // This was a notification, no response needed
                        }
                        Err(e) => {
                            error!("Error processing request: {}", e);

                            // Send error response
                            let error_response = JsonRpcResponse::error(
                                None,
                                JsonRpcError::internal_error(e.to_string()),
                            );

                            let response_str = serde_json::to_string(&error_response)?;
                            stdout.write_all(response_str.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn process_request(&self, input: &str) -> Result<Option<JsonRpcResponse>> {
        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(input) {
            Ok(req) => req,
            Err(e) => {
                warn!("Failed to parse request: {}", e);
                return Ok(Some(JsonRpcResponse::error(
                    None,
                    JsonRpcError::parse_error(),
                )));
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return Ok(Some(JsonRpcResponse::error(
                request.id.clone(),
                JsonRpcError::invalid_request(),
            )));
        }

        // Route to appropriate handler
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request).await.map(Some),
            "initialized" | "notifications/initialized" => self.handle_initialized(request).await,
            "tools/list" => self.handle_list_tools(request).await.map(Some),
            "tools/call" => self.handle_call_tool(request).await.map(Some),
            "prompts/list" => self.handle_list_prompts(request).await.map(Some),
            "resources/list" => self.handle_list_resources(request).await.map(Some),
            _ => {
                warn!("Unknown method: {}", request.method);
                Ok(Some(JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::method_not_found(&request.method),
                )))
            }
        }
    }

    async fn handle_initialize(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Parse initialize params
        let params: InitializeRequest = match request.params {
            Some(p) => serde_json::from_value(p)?,
            None => {
                return Ok(JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params("Missing params".to_string()),
                ));
            }
        };

        // Support both protocol versions
        let protocol_version = if params.protocol_version.starts_with("2025") {
            PROTOCOL_VERSION_2025.to_string()
        } else {
            PROTOCOL_VERSION.to_string()
        };

        // Create initialize result - matching ht-mcp's simpler structure
        let result = InitializeResult {
            protocol_version,
            capabilities: ServerCapabilities {
                tools: HashMap::new(), // Empty tools object like ht-mcp
                experimental: Default::default(),
            },
            server_info: ServerInfo {
                name: "Slack MCP Server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        Ok(JsonRpcResponse::success(
            request.id,
            serde_json::to_value(result)?,
        ))
    }

    async fn handle_initialized(&self, request: JsonRpcRequest) -> Result<Option<JsonRpcResponse>> {
        let mut initialized = self.initialized.write().await;
        *initialized = true;

        // Notifications don't get responses
        if request.id.is_none() {
            // This is a notification, no response needed
            Ok(None)
        } else {
            // If it has an ID, it's a request and needs a response
            Ok(Some(JsonRpcResponse::success(request.id, Value::Null)))
        }
    }

    async fn handle_list_tools(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Check if initialized
        let initialized = self.initialized.read().await;
        if !*initialized {
            return Ok(JsonRpcResponse::error(
                request.id,
                JsonRpcError::internal_error("Server not initialized".to_string()),
            ));
        }

        let tools = self.handler.list_tools().await;
        let result = ListToolsResult { tools };

        Ok(JsonRpcResponse::success(
            request.id,
            serde_json::to_value(result)?,
        ))
    }

    async fn handle_call_tool(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Check if initialized
        let initialized = self.initialized.read().await;
        if !*initialized {
            return Ok(JsonRpcResponse::error(
                request.id,
                JsonRpcError::internal_error("Server not initialized".to_string()),
            ));
        }

        // Parse call tool params
        let params: CallToolRequest = match request.params {
            Some(p) => serde_json::from_value(p)?,
            None => {
                return Ok(JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params("Missing params".to_string()),
                ));
            }
        };

        // Execute tool
        match self.handler.call_tool(&params.name, params.arguments).await {
            Ok(result) => Ok(JsonRpcResponse::success(
                request.id,
                serde_json::to_value(result)?,
            )),
            Err(e) => {
                error!("Tool execution failed: {}", e);
                Ok(JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::internal_error(e.to_string()),
                ))
            }
        }
    }

    async fn handle_list_prompts(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // We don't have prompts, return empty list
        let result = serde_json::json!({
            "prompts": []
        });

        Ok(JsonRpcResponse::success(request.id, result))
    }

    async fn handle_list_resources(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // We don't have resources, return empty list
        let result = serde_json::json!({
            "resources": []
        });

        Ok(JsonRpcResponse::success(request.id, result))
    }
}
