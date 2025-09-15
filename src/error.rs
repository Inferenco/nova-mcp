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

    #[error("Validation error: {message}")]
    ValidationError { message: String },

    #[error("Pool not found: {address}")]
    PoolNotFound { address: String },

    #[error("Token not found: {address}")]
    TokenNotFound { address: String },

    #[error("Invalid address: {address}")]
    InvalidAddress { address: String },

    #[error("Plugin not found: {plugin_id}")]
    PluginNotFound { plugin_id: u64 },

    #[error("Plugin {plugin_id} is not enabled for {context_type} {context_id}")]
    PluginNotEnabled {
        plugin_id: u64,
        context_type: String,
        context_id: String,
    },

    #[error("Storage error: {0}")]
    StorageError(#[from] sled::Error),

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

    pub fn validation_error(msg: impl Into<String>) -> Self {
        NovaError::ValidationError {
            message: msg.into(),
        }
    }

    pub fn plugin_not_found(plugin_id: u64) -> Self {
        NovaError::PluginNotFound { plugin_id }
    }

    pub fn plugin_not_enabled(
        plugin_id: u64,
        context_type: impl Into<String>,
        context_id: impl Into<String>,
    ) -> Self {
        NovaError::PluginNotEnabled {
            plugin_id,
            context_type: context_type.into(),
            context_id: context_id.into(),
        }
    }
}
