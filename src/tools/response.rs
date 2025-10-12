use crate::error::McpResult;
use serde_json::Value;

/// Simplified unified response structure for all tools
#[derive(Debug)]
pub struct ToolResponse {
    /// The actual data returned by the tool
    pub data: Value,

    /// Optional metadata (only when truly necessary)
    pub metadata: Option<ResponseMetadata>,
}

#[derive(Debug)]
pub struct ResponseMetadata {
    /// Indicates if there's more data available (for pagination)
    pub has_more: Option<bool>,

    /// Cursor for pagination
    pub next_cursor: Option<String>,

    /// Total count (only when different from returned items)
    pub total_count: Option<usize>,
}

impl ToolResponse {
    /// Create a simple response with just data
    pub fn data(data: Value) -> Self {
        Self {
            data,
            metadata: None,
        }
    }

    /// Create a response with pagination info
    pub fn paginated(data: Value, has_more: bool, next_cursor: Option<String>) -> Self {
        Self {
            data,
            metadata: Some(ResponseMetadata {
                has_more: Some(has_more),
                next_cursor,
                total_count: None,
            }),
        }
    }

    /// Convert to JSON Value for MCP protocol
    pub fn into_json(self) -> Value {
        if let Some(metadata) = self.metadata {
            let mut result = self.data;

            // Only add metadata fields that have values
            if let Some(has_more) = metadata.has_more {
                result["has_more"] = has_more.into();
            }
            if let Some(cursor) = metadata.next_cursor {
                result["next_cursor"] = cursor.into();
            }
            if let Some(count) = metadata.total_count {
                result["total_count"] = count.into();
            }

            result
        } else {
            self.data
        }
    }
}

/// Helper trait for converting tool results to responses
pub trait IntoToolResponse {
    fn into_response(self) -> McpResult<Value>;
}

impl IntoToolResponse for ToolResponse {
    fn into_response(self) -> McpResult<Value> {
        Ok(self.into_json())
    }
}
