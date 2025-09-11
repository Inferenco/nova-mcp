use serde::{Deserialize, Serialize};

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
