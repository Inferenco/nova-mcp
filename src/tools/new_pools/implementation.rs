use super::dto::{GetNewPoolsInput, GetNewPoolsOutput};
use crate::error::{NovaError, Result};
use crate::tools::gecko_terminal::helpers::build_url;
use std::time::Duration;

#[derive(Clone)]
pub struct NewPoolsTools {
    http: reqwest::Client,
    base_url: String,
}

impl NewPoolsTools {
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

    pub async fn get_new_pools(&self, input: GetNewPoolsInput) -> Result<GetNewPoolsOutput> {
        if input.network.trim().is_empty() {
            return Err(NovaError::api_error("network is required"));
        }
        let page = input.page.unwrap_or(1);
        if page == 0 || page > 10 {
            return Err(NovaError::api_error("page must be 1..=10"));
        }
        let mut url = build_url(&self.base_url, &["networks", &input.network, "new_pools"]);
        url.push_str(&format!(
            "?page={}&include=base_token,quote_token,dex",
            page
        ));
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
        Ok(GetNewPoolsOutput { pools })
    }
}

impl Default for NewPoolsTools {
    fn default() -> Self {
        Self::new()
    }
}
