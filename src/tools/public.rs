use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct PublicTools {
    http: reqwest::Client,
}

impl PublicTools {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("Nova-MCP/0.1.0")
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub async fn get_cat_fact(&self, input: GetCatFactInput) -> Result<GetCatFactOutput> {
        let mut req = self.http.get("https://catfact.ninja/fact");
        if let Some(max_length) = input.max_length {
            req = req.query(&[("max_length", max_length)]);
        }
        let resp: CatFactApi = req.send().await?.error_for_status()?.json().await?;
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
            .await?
            .error_for_status()?
            .json()
            .await?;

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

// Inputs/Outputs

#[derive(Debug, Serialize, Deserialize)]
pub struct GetCatFactInput {
    pub max_length: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetCatFactOutput {
    pub fact: String,
    pub length: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetBtcPriceInput {}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetBtcPriceOutput {
    pub usd_price: f64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub source: String,
}

// API models

#[derive(Debug, Deserialize)]
struct CatFactApi {
    fact: String,
    length: usize,
}

#[derive(Debug, Deserialize)]
struct CoingeckoApi {
    market_data: CoingeckoMarketData,
    last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
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
