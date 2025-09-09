use crate::config::NovaConfig;
use crate::error::Result;
use crate::mcp::dto::Tool;
// Re-export MCP DTOs under `server` for backward compatibility
pub use crate::mcp::dto::{McpError, McpRequest, McpResponse, ToolCall, ToolResult};
use crate::tools::public::PublicTools;
use serde_json::json;

pub struct NovaServer {
    public_tools: PublicTools,
}

impl NovaServer {
    pub fn new(_config: NovaConfig) -> Self {
        let public_tools = PublicTools::new();
        Self { public_tools }
    }

    pub fn public_tools(&self) -> &PublicTools {
        &self.public_tools
    }

    pub fn get_tools(&self) -> Vec<Tool> {
        vec![
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
        ]
    }

    // handler logic is moved into crate::mcp::handler; keep server responsibilities focused

    // Backward-compatible wrapper for tests/examples
    pub async fn handle_tool_call(&self, tool_call: ToolCall) -> Result<ToolResult> {
        crate::mcp::handler::handle_tool_call(self, tool_call).await
    }
}
