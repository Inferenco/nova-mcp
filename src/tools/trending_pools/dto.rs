use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTrendingPoolsInput {
    pub network: String,
    pub limit: Option<u32>,
    pub page: Option<u32>,
    pub duration: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTrendingPoolsOutput {
    pub pools: serde_json::Value,
}
