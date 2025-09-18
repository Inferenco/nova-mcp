use crate::config::NovaConfig;
use crate::error::Result;
use crate::mcp::dto::Tool;
use crate::plugins::{PluginManager, RequestContext};
// Re-export MCP DTOs under `server` for backward compatibility
pub use crate::mcp::dto::{McpError, McpRequest, McpResponse, ToolCall, ToolResult};
use crate::tools::gecko_terminal::GeckoTerminalTools;
use crate::tools::new_pools::NewPoolsTools;
use crate::tools::search_pools::SearchPoolsTools;
use crate::tools::trending_pools::TrendingPoolsTools;
use serde_json::json;
use std::sync::Arc;

pub struct NovaServer {
    gecko_terminal_tools: GeckoTerminalTools,
    trending_pools_tools: TrendingPoolsTools,
    search_pools_tools: SearchPoolsTools,
    new_pools_tools: NewPoolsTools,
    plugin_manager: Arc<PluginManager>,
}

impl NovaServer {
    pub fn new(_config: NovaConfig, plugin_manager: Arc<PluginManager>) -> Self {
        let gecko_terminal_tools = GeckoTerminalTools::new();
        let trending_pools_tools = TrendingPoolsTools::new();
        let search_pools_tools = SearchPoolsTools::new();
        let new_pools_tools = NewPoolsTools::new();
        Self {
            gecko_terminal_tools,
            trending_pools_tools,
            search_pools_tools,
            new_pools_tools,
            plugin_manager,
        }
    }

    pub fn gecko_terminal_tools(&self) -> &GeckoTerminalTools {
        &self.gecko_terminal_tools
    }

    pub fn trending_pools_tools(&self) -> &TrendingPoolsTools {
        &self.trending_pools_tools
    }

    pub fn search_pools_tools(&self) -> &SearchPoolsTools {
        &self.search_pools_tools
    }

    pub fn new_pools_tools(&self) -> &NewPoolsTools {
        &self.new_pools_tools
    }

    pub fn get_tools(&self, context: &RequestContext) -> Result<Vec<Tool>> {
        let mut tools = vec![];

        tools.push(Tool {
            name: "get_gecko_networks".to_string(),
            description: "List available networks from GeckoTerminal".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        });

        tools.push(Tool {
            name: "get_gecko_token".to_string(),
            description: "Fetch token info from GeckoTerminal".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "network": { "type": "string" },
                    "address": { "type": "string" }
                },
                "required": ["network", "address"],
            }),
        });

        tools.push(Tool {
            name: "get_gecko_pool".to_string(),
            description: "Fetch pool info from GeckoTerminal".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "network": { "type": "string" },
                    "address": { "type": "string" }
                },
                "required": ["network", "address"],
            }),
        });

        tools.push(Tool {
            name: "get_trending_pools".to_string(),
            description: "Fetch trending DEX pools from GeckoTerminal".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "network": { "type": "string" },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 20,
                        "default": 10
                    },
                    "page": { "type": "integer", "minimum": 1, "default": 1 },
                    "duration": {
                        "type": "string",
                        "enum": ["5m", "1h", "6h", "24h"],
                        "default": "24h"
                    }
                },
                "required": ["network"],
            }),
        });

        tools.push(Tool {
            name: "search_pools".to_string(),
            description: "Search for DEX pools on GeckoTerminal".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "network": { "type": "string" },
                    "page": { "type": "integer", "minimum": 1, "default": 1 }
                },
                "required": ["query"],
            }),
        });

        tools.push(Tool {
            name: "get_new_pools".to_string(),
            description: "Fetch newest DEX pools from GeckoTerminal".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "network": { "type": "string" },
                    "page": { "type": "integer", "minimum": 1, "default": 1 }
                },
                "required": ["network"],
            }),
        });

        let plugin_tools = self.plugin_manager.list_plugins_for_context(context)?;
        for plugin in plugin_tools {
            tools.push(Tool {
                name: plugin.fq_name,
                description: plugin.description,
                input_schema: plugin.input_schema,
            });
        }

        Ok(tools)
    }

    pub fn plugin_manager(&self) -> &PluginManager {
        self.plugin_manager.as_ref()
    }

    pub fn plugin_manager_arc(&self) -> Arc<PluginManager> {
        Arc::clone(&self.plugin_manager)
    }

    // handler logic is moved into crate::mcp::handler; keep server responsibilities focused

    // Backward-compatible wrapper for tests/examples
    pub async fn handle_tool_call(
        &self,
        tool_call: ToolCall,
        context: &RequestContext,
    ) -> Result<ToolResult> {
        crate::mcp::handler::handle_tool_call(self, tool_call, context).await
    }
}
