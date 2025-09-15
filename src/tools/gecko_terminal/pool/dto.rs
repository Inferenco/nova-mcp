use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoPoolInput {
    pub network: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoPoolOutput {
    pub pool: serde_json::Value,
}
