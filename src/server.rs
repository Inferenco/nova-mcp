use crate::config::NovaConfig;
use crate::error::Result;
use crate::mcp::dto::Tool;
// Re-export MCP DTOs under `server` for backward compatibility
pub use crate::mcp::dto::{McpError, McpRequest, McpResponse, ToolCall, ToolResult};
use crate::tools::gecko_terminal::GeckoTerminalTools;
use crate::tools::public::PublicTools;
use serde_json::json;

pub struct NovaServer {
    public_tools: PublicTools,
    gecko_terminal_tools: GeckoTerminalTools,
}

impl NovaServer {
    pub fn new(_config: NovaConfig) -> Self {
        let public_tools = PublicTools::new();
        let gecko_terminal_tools = GeckoTerminalTools::new();
        Self {
            public_tools,
            gecko_terminal_tools,
        }
    }

    pub fn public_tools(&self) -> &PublicTools {
        &self.public_tools
    }

    pub fn gecko_terminal_tools(&self) -> &GeckoTerminalTools {
        &self.gecko_terminal_tools
    }

    pub fn get_tools(&self) -> Vec<Tool> {
        let mut tools = vec![
            Tool {
                name: "get_cat_fact".to_string(),
                description: "Fetch a random cat fact (catfact.ninja)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "max_length": { "type": "number", "description": "Optional maximum length of fact" }
                    }
                }),
            },
            Tool {
                name: "get_btc_price".to_string(),
                description: "Fetch current BTC price in USD (CoinGecko)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ];

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

        tools
    }

    // handler logic is moved into crate::mcp::handler; keep server responsibilities focused

    // Backward-compatible wrapper for tests/examples
    pub async fn handle_tool_call(&self, tool_call: ToolCall) -> Result<ToolResult> {
        crate::mcp::handler::handle_tool_call(self, tool_call).await
    }
}
