use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GetGeckoNetworksInput {}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGeckoNetworksOutput {
    pub networks: serde_json::Value,
}
