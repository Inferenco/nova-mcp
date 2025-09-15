use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoTokenInput {
    pub network: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoTokenOutput {
    pub token: serde_json::Value,
}
