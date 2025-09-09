use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct PublicTools {
    http: reqwest::Client,
}

impl PublicTools {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
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
        let resp: CoindeskApi = self
            .http
            .get("https://api.coindesk.com/v1/bpi/currentprice.json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let price = resp.bpi.get("USD").map(|usd| usd.rate_float).unwrap_or(0.0);

        Ok(GetBtcPriceOutput {
            usd_price: price,
            updated_at: resp.time.updated_iso,
            source: "coindesk".to_string(),
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
struct CoindeskApi {
    time: CoindeskTime,
    bpi: std::collections::HashMap<String, CoindeskBpi>,
}

#[derive(Debug, Deserialize)]
struct CoindeskTime {
    #[serde(rename = "updatedISO")]
    updated_iso: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct CoindeskBpi {
    #[allow(dead_code)]
    code: String,
    #[allow(dead_code)]
    rate: String,
    rate_float: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_coindesk_sample() {
        let sample = r#"{
            "time": { "updatedISO": "2024-01-01T00:00:00Z" },
            "bpi": {
              "USD": { "code": "USD", "rate": "100,000.0000", "rate_float": 100000.0 }
            }
        }"#;
        let parsed: CoindeskApi = serde_json::from_str(sample).unwrap();
        assert_eq!(
            parsed.time.updated_iso.to_rfc3339(),
            "2024-01-01T00:00:00+00:00"
        );
        assert_eq!(parsed.bpi.get("USD").unwrap().rate_float, 100000.0);
    }
}
