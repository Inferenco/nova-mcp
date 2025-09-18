use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistrationRequest {
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub scopes: Vec<String>,
    pub endpoint: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub trust_level: String,
    #[serde(default)]
    pub context_type: Option<PluginContextType>,
    #[serde(default)]
    pub context_id: Option<String>,
    #[serde(default)]
    pub input_schema: Option<Value>,
    #[serde(default)]
    pub output_schema: Option<Value>,
    #[serde(default)]
    pub version: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub plugin_id: u64,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub scopes: Vec<String>,
    pub endpoint: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub trust_level: String,
    #[serde(default)]
    pub context_type: Option<PluginContextType>,
    #[serde(default)]
    pub context_id: Option<String>,
    #[serde(default)]
    pub input_schema: Option<Value>,
    #[serde(default)]
    pub output_schema: Option<Value>,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub fully_qualified_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginUpdateRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub icon_url: Option<Option<String>>,
    #[serde(default)]
    pub trust_level: Option<String>,
    #[serde(default)]
    pub input_schema: Option<Option<Value>>,
    #[serde(default)]
    pub output_schema: Option<Option<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolUpdateRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub input_schema: Option<Value>,
    #[serde(default)]
    pub output_schema: Option<Option<Value>>,
    #[serde(default)]
    pub icon_url: Option<Option<String>>,
    #[serde(default)]
    pub trust_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRegistrationResponse {
    pub plugin_id: u64,
    pub fully_qualified_name: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginContextType {
    User,
    Group,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInvocationRequest {
    pub context_type: PluginContextType,
    pub context_id: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInvocationPayload {
    pub context_type: PluginContextType,
    pub context_id: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEnableRequest {
    pub context_type: PluginContextType,
    pub context_id: String,
    pub plugin_id: u64,
    pub enable: bool,
    #[serde(default)]
    pub added_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEnablementStatus {
    pub context_type: PluginContextType,
    pub context_id: String,
    pub plugin_id: u64,
    pub enabled: bool,
    pub consent_ts: i64,
    #[serde(default)]
    pub added_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPluginRecord {
    pub enabled: bool,
    pub consent_ts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupPluginRecord {
    pub enabled: bool,
    #[serde(default)]
    pub added_by: Option<String>,
    pub consent_ts: i64,
}

fn default_version() -> u32 {
    1
}
