use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPoolsInput {
    pub query: String,
    pub network: Option<String>,
    pub page: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchPoolsOutput {
    pub pools: serde_json::Value,
}
