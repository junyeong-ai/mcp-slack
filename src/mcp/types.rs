use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// MCP Protocol versions
pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const PROTOCOL_VERSION_2025: &str = "2025-06-18";

/// JSON-RPC Request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

/// JSON-RPC Response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

/// JSON-RPC Error
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP Initialize Request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitializeRequest {
    #[serde(alias = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(alias = "clientInfo")]
    pub client_info: ClientInfo,
}

/// Client Capabilities
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub experimental: HashMap<String, Value>,
}

/// Client Information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Initialize Result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

/// Server Capabilities
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerCapabilities {
    pub tools: HashMap<String, Value>,
    #[serde(default)]
    pub experimental: HashMap<String, Value>,
}

/// Server Information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Tool Definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: ToolInputSchema,
}

/// Tool Input Schema
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, Property>,
    #[serde(default)]
    pub required: Vec<String>,
}

/// Property Definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Property {
    #[serde(rename = "type")]
    pub property_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<Value>>,
}

/// List Tools Result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
}

/// Call Tool Request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallToolRequest {
    pub name: String,
    pub arguments: Value,
}

/// Call Tool Result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallToolResult {
    pub content: Vec<ToolContent>,
}

/// Tool Content
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

/// MCP Error Codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

impl JsonRpcError {
    pub fn parse_error() -> Self {
        Self {
            code: error_codes::PARSE_ERROR,
            message: "Parse error".to_string(),
            data: None,
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            code: error_codes::INVALID_REQUEST,
            message: "Invalid request".to_string(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: error_codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    pub fn invalid_params(message: String) -> Self {
        Self {
            code: error_codes::INVALID_PARAMS,
            message,
            data: None,
        }
    }

    pub fn internal_error(message: String) -> Self {
        Self {
            code: error_codes::INTERNAL_ERROR,
            message,
            data: None,
        }
    }
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Option<Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}
