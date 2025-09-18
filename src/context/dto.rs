use serde::{Deserialize, Serialize};

use crate::plugins::PluginContextType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestContext {
    pub context_type: PluginContextType,
    pub context_id: String,
}
