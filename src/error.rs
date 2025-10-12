use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for McpError {
    fn from(err: anyhow::Error) -> Self {
        McpError::Internal(err.to_string())
    }
}

pub type McpResult<T> = std::result::Result<T, McpError>;

/// Extension trait for converting errors to McpError with context
pub trait IntoMcpError<T> {
    fn mcp_context(self, context: &str) -> McpResult<T>;
}

impl<T, E: std::fmt::Display> IntoMcpError<T> for Result<T, E> {
    fn mcp_context(self, context: &str) -> McpResult<T> {
        self.map_err(|e| McpError::Internal(format!("{}: {}", context, e)))
    }
}
