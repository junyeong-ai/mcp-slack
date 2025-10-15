use std::fmt;

/// Cache-specific error types
#[derive(Debug)]
pub enum CacheError {
    /// Failed to acquire a connection from the pool
    ConnectionPoolError(r2d2::Error),

    /// Database operation failed
    DatabaseError(rusqlite::Error),

    /// JSON serialization/deserialization failed
    SerializationError(serde_json::Error),

    /// Failed to acquire distributed lock after retries
    LockAcquisitionFailed { key: String, attempts: usize },

    /// System time error (used in lock operations)
    SystemTimeError(std::time::SystemTimeError),

    /// Invalid input data (e.g., empty vectors)
    InvalidInput(String),
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::ConnectionPoolError(e) => {
                write!(f, "Failed to get connection from pool: {}", e)
            }
            CacheError::DatabaseError(e) => {
                write!(f, "Database operation failed: {}", e)
            }
            CacheError::SerializationError(e) => {
                write!(f, "JSON serialization failed: {}", e)
            }
            CacheError::LockAcquisitionFailed { key, attempts } => {
                write!(
                    f,
                    "Failed to acquire lock for '{}' after {} attempts",
                    key, attempts
                )
            }
            CacheError::SystemTimeError(e) => {
                write!(f, "System time error: {}", e)
            }
            CacheError::InvalidInput(msg) => {
                write!(f, "Invalid input: {}", msg)
            }
        }
    }
}

impl std::error::Error for CacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CacheError::ConnectionPoolError(e) => Some(e),
            CacheError::DatabaseError(e) => Some(e),
            CacheError::SerializationError(e) => Some(e),
            CacheError::SystemTimeError(e) => Some(e),
            CacheError::LockAcquisitionFailed { .. } => None,
            CacheError::InvalidInput(_) => None,
        }
    }
}

// Implement From conversions for common error types
impl From<r2d2::Error> for CacheError {
    fn from(err: r2d2::Error) -> Self {
        CacheError::ConnectionPoolError(err)
    }
}

impl From<rusqlite::Error> for CacheError {
    fn from(err: rusqlite::Error) -> Self {
        CacheError::DatabaseError(err)
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        CacheError::SerializationError(err)
    }
}

impl From<std::time::SystemTimeError> for CacheError {
    fn from(err: std::time::SystemTimeError) -> Self {
        CacheError::SystemTimeError(err)
    }
}

impl From<std::io::Error> for CacheError {
    fn from(err: std::io::Error) -> Self {
        // IO errors during cache operations are considered database errors
        CacheError::DatabaseError(rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
    }
}

pub type CacheResult<T> = Result<T, CacheError>;
