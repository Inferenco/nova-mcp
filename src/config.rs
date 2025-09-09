use crate::error::{NovaError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaConfig {
    pub server: ServerConfig,
    pub apis: ApiConfig,
    pub cache: CacheConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub log_level: String,
    pub transport: String, // "stdio", "sse", "http"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub uniswap_api_key: Option<String>,
    pub coingecko_api_key: Option<String>,
    pub dexscreener_api_key: Option<String>,
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub ttl_seconds: u64,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    // Comma-separated API keys via env; for production replace with hashed store
    pub allowed_keys: Vec<String>,
    pub header_name: String,
}

impl Default for NovaConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                port: 8080,
                log_level: "info".to_string(),
                transport: "stdio".to_string(),
            },
            apis: ApiConfig {
                uniswap_api_key: None,
                coingecko_api_key: None,
                dexscreener_api_key: None,
                rate_limit_per_minute: 60,
            },
            cache: CacheConfig {
                ttl_seconds: 300,
                max_entries: 1000,
            },
            auth: AuthConfig {
                enabled: false,
                allowed_keys: vec![],
                header_name: "x-api-key".to_string(),
            },
        }
    }
}

impl NovaConfig {
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(port) = std::env::var("NOVA_MCP_PORT") {
            config.server.port = port
                .parse()
                .map_err(|_| NovaError::config_error("Invalid NOVA_MCP_PORT"))?;
        }

        if let Ok(log_level) = std::env::var("NOVA_MCP_LOG_LEVEL") {
            config.server.log_level = log_level;
        }

        if let Ok(transport) = std::env::var("NOVA_MCP_TRANSPORT") {
            config.server.transport = transport;
        }

        config.apis.uniswap_api_key = std::env::var("UNISWAP_API_KEY").ok();
        config.apis.coingecko_api_key = std::env::var("COINGECKO_API_KEY").ok();
        config.apis.dexscreener_api_key = std::env::var("DEXSCREENER_API_KEY").ok();

        // Auth configuration
        if let Ok(enabled) = std::env::var("NOVA_MCP_AUTH_ENABLED") {
            config.auth.enabled = matches!(enabled.as_str(), "1" | "true" | "TRUE" | "yes" | "on");
        }
        if let Ok(keys) = std::env::var("NOVA_MCP_API_KEYS") {
            let list = keys
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            if !list.is_empty() {
                config.auth.allowed_keys = list;
            }
        }
        if let Ok(header_name) = std::env::var("NOVA_MCP_AUTH_HEADER") {
            if !header_name.trim().is_empty() {
                config.auth.header_name = header_name;
            }
        }

        Ok(config)
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| NovaError::config_error(format!("Failed to read config file: {}", e)))?;

        let config: NovaConfig = toml::from_str(&content)
            .map_err(|e| NovaError::config_error(format!("Failed to parse config file: {}", e)))?;

        Ok(config)
    }
}
