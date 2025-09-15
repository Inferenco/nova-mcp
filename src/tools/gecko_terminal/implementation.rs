use super::helpers::build_url;
use super::networks::dto::{GetGeckoNetworksInput, GetGeckoNetworksOutput};
use super::pool::dto::{GetGeckoPoolInput, GetGeckoPoolOutput};
use super::token::dto::{GetGeckoTokenInput, GetGeckoTokenOutput};
use crate::error::{NovaError, Result};
use std::time::Duration;

#[derive(Clone)]
pub struct GeckoTerminalTools {
    http: reqwest::Client,
    base_url: String,
}

impl GeckoTerminalTools {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Nova-MCP/0.1.0")
            .build()
            .unwrap_or_else(|e| {
                tracing::error!("Failed to build HTTP client: {}", e);
                reqwest::Client::new()
            });
        Self {
            http,
            base_url: "https://api.geckoterminal.com/api/v2".to_string(),
        }
    }

    pub async fn get_networks(
        &self,
        _input: GetGeckoNetworksInput,
    ) -> Result<GetGeckoNetworksOutput> {
        let url = build_url(&self.base_url, &["networks"]);
        let networks = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(NovaError::NetworkError)?
            .error_for_status()
            .map_err(NovaError::NetworkError)?
            .json::<serde_json::Value>()
            .await
            .map_err(NovaError::NetworkError)?;
        Ok(GetGeckoNetworksOutput { networks })
    }

    pub async fn get_token(&self, input: GetGeckoTokenInput) -> Result<GetGeckoTokenOutput> {
        let url = build_url(
            &self.base_url,
            &["networks", &input.network, "tokens", &input.address],
        );
        let token = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(NovaError::NetworkError)?
            .error_for_status()
            .map_err(NovaError::NetworkError)?
            .json::<serde_json::Value>()
            .await
            .map_err(NovaError::NetworkError)?;
        Ok(GetGeckoTokenOutput { token })
    }

    pub async fn get_pool(&self, input: GetGeckoPoolInput) -> Result<GetGeckoPoolOutput> {
        let url = build_url(
            &self.base_url,
            &["networks", &input.network, "pools", &input.address],
        );
        let pool = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(NovaError::NetworkError)?
            .error_for_status()
            .map_err(NovaError::NetworkError)?
            .json::<serde_json::Value>()
            .await
            .map_err(NovaError::NetworkError)?;
        Ok(GetGeckoPoolOutput { pool })
    }
}

impl Default for GeckoTerminalTools {
    fn default() -> Self {
        Self::new()
    }
}
