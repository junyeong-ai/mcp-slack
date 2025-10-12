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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_response_data_only() {
        let data = json!({"key": "value", "count": 42});
        let response = ToolResponse::data(data.clone());

        assert!(response.metadata.is_none());
        let result = response.into_json();
        assert_eq!(result["key"], "value");
        assert_eq!(result["count"], 42);
        assert!(result.get("has_more").is_none());
    }

    #[test]
    fn test_tool_response_paginated_with_cursor() {
        let data = json!({"items": [1, 2, 3]});
        let response =
            ToolResponse::paginated(data.clone(), true, Some("next_page_token".to_string()));

        let result = response.into_json();
        assert_eq!(result["items"], json!([1, 2, 3]));
        assert_eq!(result["has_more"], json!(true));
        assert_eq!(result["next_cursor"], "next_page_token");
    }

    #[test]
    fn test_tool_response_paginated_without_cursor() {
        let data = json!({"items": [1, 2, 3]});
        let response = ToolResponse::paginated(data.clone(), false, None);

        let result = response.into_json();
        assert_eq!(result["items"], json!([1, 2, 3]));
        assert_eq!(result["has_more"], json!(false));
        assert!(result.get("next_cursor").is_none());
    }

    #[test]
    fn test_tool_response_metadata_fields_only_when_present() {
        let data = json!({"result": "ok"});
        let response = ToolResponse {
            data,
            metadata: Some(ResponseMetadata {
                has_more: Some(true),
                next_cursor: None,
                total_count: Some(100),
            }),
        };

        let result = response.into_json();
        assert_eq!(result["has_more"], json!(true));
        assert!(result.get("next_cursor").is_none());
        assert_eq!(result["total_count"], 100);
    }

    #[test]
    fn test_into_tool_response_trait() {
        let data = json!({"status": "success"});
        let response = ToolResponse::data(data);
        let result = response.into_response();

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["status"], "success");
    }

    #[test]
    fn test_tool_response_empty_data() {
        let data = json!({});
        let response = ToolResponse::data(data);
        let result = response.into_json();

        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_tool_response_array_data() {
        let data = json!([1, 2, 3, 4, 5]);
        let response = ToolResponse::data(data);
        let result = response.into_json();

        assert_eq!(result, json!([1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_tool_response_paginated_preserves_nested_structure() {
        let data = json!({
            "users": [
                {"id": "U1", "name": "Alice"},
                {"id": "U2", "name": "Bob"}
            ],
            "total": 50
        });
        let response = ToolResponse::paginated(data, true, Some("cursor_abc".to_string()));

        let result = response.into_json();
        assert_eq!(result["users"][0]["name"], "Alice");
        assert_eq!(result["total"], 50);
        assert_eq!(result["has_more"], json!(true));
        assert_eq!(result["next_cursor"], "cursor_abc");
    }

    #[test]
    fn test_response_metadata_all_none() {
        let data = json!({"data": "test"});
        let response = ToolResponse {
            data,
            metadata: Some(ResponseMetadata {
                has_more: None,
                next_cursor: None,
                total_count: None,
            }),
        };

        let result = response.into_json();
        assert_eq!(result["data"], "test");
        assert!(result.get("has_more").is_none());
        assert!(result.get("next_cursor").is_none());
        assert!(result.get("total_count").is_none());
    }
}
