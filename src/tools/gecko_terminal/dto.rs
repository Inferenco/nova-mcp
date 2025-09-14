use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GetGeckoNetworksInput {}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoNetworksOutput {
    pub networks: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoTokenInput {
    pub network: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoTokenOutput {
    pub token: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoPoolInput {
    pub network: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoPoolOutput {
    pub pool: serde_json::Value,
}
