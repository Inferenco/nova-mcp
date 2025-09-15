use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetNewPoolsInput {
    pub network: String,
    pub page: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetNewPoolsOutput {
    pub pools: serde_json::Value,
}
