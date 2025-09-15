use crate::server::NovaServer;
use crate::{
    error::NovaError,
    tools::gecko_terminal::{
        get_networks, get_pool, get_token, GetGeckoNetworksInput, GetGeckoPoolInput,
        GetGeckoTokenInput,
    },
    tools::new_pools::{get_new_pools, GetNewPoolsInput},
    tools::public::{GetBtcPriceInput, GetCatFactInput},
    tools::search_pools::{search_pools, SearchPoolsInput},
    tools::trending_pools::{get_trending_pools, GetTrendingPoolsInput},
};
use serde_json::json;

use super::dto::{McpError, McpRequest, McpResponse, ToolCall, ToolResult};

pub async fn handle_request(server: &NovaServer, request: McpRequest) -> McpResponse {
    match request.method.as_str() {
        "tools/list" => {
            let tools = server.get_tools();
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
                    match handle_tool_call(server, tool_call).await {
                        Ok(result) => McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: Some(json!({
                                "content": [
                                    { "type": "text", "text": result.content }
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
                        },
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
        "initialize" => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "nova-mcp", "version": "0.1.0" }
            })),
            error: None,
        },
        "ping" => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({ "ok": true })),
            error: None,
        },
        _ => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(McpError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
                data: None,
            }),
        },
    }
}

pub(crate) async fn handle_tool_call(
    server: &NovaServer,
    tool_call: ToolCall,
) -> Result<ToolResult, NovaError> {
    tracing::info!("Handling tool call: {}", tool_call.name);
    let result = match tool_call.name.as_str() {
        "get_cat_fact" => {
            let input: GetCatFactInput = serde_json::from_value(tool_call.arguments)?;
            // Basic input validation
            if let Some(max_len) = input.max_length {
                if max_len == 0 || max_len > 1000 {
                    return Err(NovaError::api_error("max_length must be 1..=1000"));
                }
            }
            let output = server.public_tools().get_cat_fact(input).await?;
            serde_json::to_value(output)?
        }
        "get_btc_price" => {
            let input: GetBtcPriceInput = match serde_json::from_value(tool_call.arguments) {
                Ok(v) => v,
                Err(_) => return Err(NovaError::api_error("Invalid arguments")),
            };
            let output = server.public_tools().get_btc_price(input).await?;
            serde_json::to_value(output)?
        }
        "get_gecko_networks" => {
            let input: GetGeckoNetworksInput = match serde_json::from_value(tool_call.arguments) {
                Ok(v) => v,
                Err(_) => return Err(NovaError::api_error("Invalid arguments")),
            };
            let output = get_networks(server.gecko_terminal_tools(), input).await?;
            serde_json::to_value(output)?
        }
        "get_gecko_token" => {
            let input: GetGeckoTokenInput = match serde_json::from_value(tool_call.arguments) {
                Ok(v) => v,
                Err(_) => return Err(NovaError::api_error("Invalid arguments")),
            };
            if input.network.trim().is_empty() || input.address.trim().is_empty() {
                return Err(NovaError::api_error("network and address are required"));
            }
            let output = get_token(server.gecko_terminal_tools(), input).await?;
            serde_json::to_value(output)?
        }
        "get_gecko_pool" => {
            let input: GetGeckoPoolInput = match serde_json::from_value(tool_call.arguments) {
                Ok(v) => v,
                Err(_) => return Err(NovaError::api_error("Invalid arguments")),
            };
            if input.network.trim().is_empty() || input.address.trim().is_empty() {
                return Err(NovaError::api_error("network and address are required"));
            }
            let output = get_pool(server.gecko_terminal_tools(), input).await?;
            serde_json::to_value(output)?
        }
        "get_trending_pools" => {
            let input: GetTrendingPoolsInput = match serde_json::from_value(tool_call.arguments) {
                Ok(v) => v,
                Err(_) => return Err(NovaError::api_error("Invalid arguments")),
            };
            if input.network.trim().is_empty() {
                return Err(NovaError::api_error("network is required"));
            }
            let output = get_trending_pools(server.trending_pools_tools(), input).await?;
            serde_json::to_value(output)?
        }
        "search_pools" => {
            let input: SearchPoolsInput = match serde_json::from_value(tool_call.arguments) {
                Ok(v) => v,
                Err(_) => return Err(NovaError::api_error("Invalid arguments")),
            };
            if input.query.trim().is_empty() {
                return Err(NovaError::api_error("query is required"));
            }
            let output = search_pools(server.search_pools_tools(), input).await?;
            serde_json::to_value(output)?
        }
        "get_new_pools" => {
            let input: GetNewPoolsInput = match serde_json::from_value(tool_call.arguments) {
                Ok(v) => v,
                Err(_) => return Err(NovaError::api_error("Invalid arguments")),
            };
            if input.network.trim().is_empty() {
                return Err(NovaError::api_error("network is required"));
            }
            let output = get_new_pools(server.new_pools_tools(), input).await?;
            serde_json::to_value(output)?
        }
        _ => {
            return Err(NovaError::api_error(format!(
                "Unknown tool: {}",
                tool_call.name
            )));
        }
    };

    Ok(ToolResult {
        content: serde_json::to_string_pretty(&result)?,
        is_error: false,
    })
}
