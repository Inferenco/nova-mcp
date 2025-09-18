use serde::{Deserialize, Serialize};

const fn default_plugin_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistrationRequest {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub owner_id: Option<String>,
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    pub endpoint_url: String,
    #[serde(default = "default_plugin_version")]
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginUpdateRequest {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub output_schema: Option<Option<serde_json::Value>>,
    #[serde(default)]
    pub endpoint_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PluginContextType {
    User,
    Group,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RequestContext {
    pub context_type: PluginContextType,
    pub context_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub plugin_id: u64,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub owner_id: Option<String>,
    pub context_type: PluginContextType,
    pub context_id: String,
    pub fq_name: String,
    pub version: u32,
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    pub endpoint_url: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginInvocationRequest {
    #[serde(default)]
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInvocationPayload {
    pub context_type: PluginContextType,
    pub context_id: String,
    pub arguments: serde_json::Value,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersionRecord {
    pub version: u32,
    pub fq_name: String,
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    pub endpoint_url: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPluginRecord {
    pub plugin_id: u64,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub owner_id: Option<String>,
    pub context_type: PluginContextType,
    pub context_id: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub versions: Vec<PluginVersionRecord>,
}
