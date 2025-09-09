use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use crate::config::NovaConfig;
use crate::tools::public::{PublicTools, GetCatFactInput, GetBtcPriceInput};
use crate::error::{NovaError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

pub struct NovaServer {
    config: NovaConfig,
    public_tools: PublicTools,
}

impl NovaServer {
    pub fn new(config: NovaConfig) -> Self {
        let public_tools = PublicTools::new();
        Self { config, public_tools }
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
                description: "Fetch current BTC price in USD (Coindesk)".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }
    
    pub async fn handle_tool_call(&self, tool_call: ToolCall) -> Result<ToolResult> {
        tracing::info!("Handling tool call: {}", tool_call.name);
        
        let result = match tool_call.name.as_str() {
            "get_cat_fact" => {
                let input: GetCatFactInput = serde_json::from_value(tool_call.arguments)?;
                let output = self.public_tools.get_cat_fact(input).await?;
                serde_json::to_value(output)?
            }
            "get_btc_price" => {
                let _input: GetBtcPriceInput = serde_json::from_value(tool_call.arguments).unwrap_or(GetBtcPriceInput{});
                let output = self.public_tools.get_btc_price(GetBtcPriceInput{}).await?;
                serde_json::to_value(output)?
            }
            _ => {
                return Err(NovaError::api_error(format!("Unknown tool: {}", tool_call.name)));
            }
        };
        
        Ok(ToolResult {
            content: serde_json::to_string_pretty(&result)?,
            is_error: false,
        })
    }
    
    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "tools/list" => {
                let tools = self.get_tools();
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "tools": tools
                    })),
                    error: None,
                }
            }
            "tools/call" => {
                if let Some(params) = request.params {
                    if let Ok(tool_call) = serde_json::from_value::<ToolCall>(params) {
                        match self.handle_tool_call(tool_call).await {
                            Ok(result) => McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: request.id,
                                result: Some(json!({
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": result.content
                                        }
                                    ],
                                    "isError": result.is_error
                                })),
                                error: None,
                            },
                            Err(e) => McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: request.id,
                                result: None,
                                error: Some(McpError {
                                    code: -32603,
                                    message: format!("Tool execution failed: {}", e),
                                    data: None,
                                }),
                            }
                        }
                    } else {
                        McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: None,
                            error: Some(McpError {
                                code: -32602,
                                message: "Invalid tool call parameters".to_string(),
                                data: None,
                            }),
                        }
                    }
                } else {
                    McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(McpError {
                            code: -32602,
                            message: "Missing parameters".to_string(),
                            data: None,
                        }),
                    }
                }
            }
            "initialize" => {
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "nova-mcp",
                            "version": "0.1.0"
                        }
                    })),
                    error: None,
                }
            }
            "ping" => {
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({ "ok": true })),
                    error: None,
                }
            }
            _ => {
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    }),
                }
            }
        }
    }
}
