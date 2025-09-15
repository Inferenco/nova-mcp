use super::dto::{SearchPoolsInput, SearchPoolsOutput};
use crate::error::{NovaError, Result};
use std::time::Duration;
use urlencoding::encode;

#[derive(Clone)]
pub struct SearchPoolsTools {
    http: reqwest::Client,
    base_url: String,
}

impl SearchPoolsTools {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Nova-MCP/0.1.0")
            .build()
            .unwrap_or_else(|e| {
                tracing::error!("Failed to build HTTP client: {}", e);
                reqwest::Client::new()
            });
        let base_url = std::env::var("GECKO_TERMINAL_BASE_URL")
            .unwrap_or_else(|_| "https://api.geckoterminal.com/api/v2".to_string());
        Self { http, base_url }
    }

    pub async fn search_pools(&self, input: SearchPoolsInput) -> Result<SearchPoolsOutput> {
        if input.query.trim().is_empty() {
            return Err(NovaError::api_error("query is required"));
        }
        let page = input.page.unwrap_or(1);
        if page == 0 || page > 10 {
            return Err(NovaError::api_error("page must be 1..=10"));
        }
        let mut url = format!(
            "{}/search/pools?query={}&page={}",
            self.base_url.trim_end_matches('/'),
            encode(&input.query),
            page
        );
        if let Some(network) = input.network {
            if !network.trim().is_empty() {
                url.push_str(&format!("&network={}", network));
            }
        }
        url.push_str("&include=base_token,quote_token,dex");
        let pools = self
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
        Ok(SearchPoolsOutput { pools })
    }
}

impl Default for SearchPoolsTools {
    fn default() -> Self {
        Self::new()
    }
}
