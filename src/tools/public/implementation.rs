use super::dto::{GetBtcPriceInput, GetBtcPriceOutput, GetCatFactInput, GetCatFactOutput};
use crate::error::{NovaError, Result};
use std::time::Duration;

#[derive(Clone)]
pub struct PublicTools {
    http: reqwest::Client,
}

impl PublicTools {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Nova-MCP/0.1.0")
            .build()
            .unwrap_or_else(|e| {
                tracing::error!("Failed to build HTTP client: {}", e);
                reqwest::Client::new()
            });
        Self { http }
    }

    pub async fn get_cat_fact(&self, input: GetCatFactInput) -> Result<GetCatFactOutput> {
        let mut req = self.http.get("https://catfact.ninja/fact");
        if let Some(max_length) = input.max_length {
            req = req.query(&[("max_length", max_length)]);
        }
        let resp: CatFactApi = req
            .send()
            .await
            .map_err(NovaError::NetworkError)?
            .error_for_status()
            .map_err(NovaError::NetworkError)?
            .json()
            .await
            .map_err(NovaError::NetworkError)?;
        Ok(GetCatFactOutput {
            fact: resp.fact,
            length: resp.length,
        })
    }

    pub async fn get_btc_price(&self, _input: GetBtcPriceInput) -> Result<GetBtcPriceOutput> {
        let resp: CoingeckoApi = self
            .http
            .get("https://api.coingecko.com/api/v3/coins/bitcoin")
            .query(&[
                ("localization", "false"),
                ("tickers", "false"),
                ("market_data", "true"),
                ("community_data", "false"),
                ("developer_data", "false"),
                ("sparkline", "false"),
            ])
            .send()
            .await
            .map_err(NovaError::NetworkError)?
            .error_for_status()
            .map_err(NovaError::NetworkError)?
            .json()
            .await
            .map_err(NovaError::NetworkError)?;

        let price = resp
            .market_data
            .current_price
            .get("usd")
            .copied()
            .unwrap_or(0.0);

        Ok(GetBtcPriceOutput {
            usd_price: price,
            updated_at: resp.last_updated,
            source: "coingecko".to_string(),
        })
    }
}

impl Default for PublicTools {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Deserialize)]
struct CatFactApi {
    fact: String,
    length: usize,
}

#[derive(Debug, serde::Deserialize)]
struct CoingeckoApi {
    market_data: CoingeckoMarketData,
    last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Deserialize)]
struct CoingeckoMarketData {
    current_price: std::collections::HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_coingecko_sample() {
        let sample = r#"{
            "last_updated": "2024-01-01T00:00:00Z",
            "market_data": {
              "current_price": { "usd": 100000.0 }
            }
        }"#;
        let parsed: CoingeckoApi = serde_json::from_str(sample).unwrap();
        assert_eq!(
            parsed.last_updated.to_rfc3339(),
            "2024-01-01T00:00:00+00:00"
        );
        assert_eq!(
            *parsed.market_data.current_price.get("usd").unwrap(),
            100000.0
        );
    }
}
