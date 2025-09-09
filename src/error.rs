use thiserror::Error;

pub type Result<T> = std::result::Result<T, NovaError>;

#[derive(Error, Debug)]
pub enum NovaError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Pool not found: {address}")]
    PoolNotFound { address: String },

    #[error("Token not found: {address}")]
    TokenNotFound { address: String },

    #[error("Invalid address: {address}")]
    InvalidAddress { address: String },

    #[error("Rate limit exceeded for API: {api}")]
    RateLimitExceeded { api: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl NovaError {
    pub fn api_error(msg: impl Into<String>) -> Self {
        NovaError::ApiError(msg.into())
    }

    pub fn config_error(msg: impl Into<String>) -> Self {
        NovaError::ConfigError(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        NovaError::Internal(msg.into())
    }
}
