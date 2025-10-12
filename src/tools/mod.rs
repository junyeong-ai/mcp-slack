pub mod cache;
pub mod message_utils;
pub mod messages;
pub mod response;
pub mod search;

use crate::error::McpResult;
use async_trait::async_trait;
use serde_json::Value;

pub use response::{IntoToolResponse, ToolResponse};

#[async_trait]
pub trait Tool {
    fn description(&self) -> &str;
    async fn execute(&self, params: Value) -> McpResult<Value>;
}
