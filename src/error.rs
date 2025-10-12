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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let mcp_err: McpError = io_err.into();
        assert!(matches!(mcp_err, McpError::Io(_)));
        assert!(mcp_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<i32>("invalid").unwrap_err();
        let mcp_err: McpError = json_err.into();
        assert!(matches!(mcp_err, McpError::Serialization(_)));
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("something went wrong");
        let mcp_err: McpError = anyhow_err.into();
        assert!(matches!(mcp_err, McpError::Internal(_)));
        assert!(mcp_err.to_string().contains("something went wrong"));
    }

    #[test]
    fn test_invalid_parameter_error() {
        let err = McpError::InvalidParameter("test param".to_string());
        assert!(err.to_string().contains("Invalid parameter"));
        assert!(err.to_string().contains("test param"));
    }

    #[test]
    fn test_not_found_error() {
        let err = McpError::NotFound("resource".to_string());
        assert!(err.to_string().contains("Not found"));
        assert!(err.to_string().contains("resource"));
    }

    #[test]
    fn test_internal_error() {
        let err = McpError::Internal("internal issue".to_string());
        assert!(err.to_string().contains("Internal error"));
        assert!(err.to_string().contains("internal issue"));
    }

    #[test]
    fn test_mcp_context_ok() {
        let result: Result<i32, String> = Ok(42);
        let mcp_result = result.mcp_context("test context");
        assert!(mcp_result.is_ok());
        assert_eq!(mcp_result.unwrap(), 42);
    }

    #[test]
    fn test_mcp_context_err() {
        let result: Result<i32, String> = Err("original error".to_string());
        let mcp_result = result.mcp_context("test context");
        assert!(mcp_result.is_err());
        let err = mcp_result.unwrap_err();
        assert!(matches!(err, McpError::Internal(_)));
        assert!(err.to_string().contains("test context"));
        assert!(err.to_string().contains("original error"));
    }

    #[test]
    fn test_mcp_context_preserves_context_format() {
        let result: Result<(), &str> = Err("error message");
        let mcp_result = result.mcp_context("Operation failed");
        let err = mcp_result.unwrap_err();
        let err_string = err.to_string();
        assert!(err_string.contains("Operation failed: error message"));
    }
}
