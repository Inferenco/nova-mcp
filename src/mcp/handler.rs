use crate::plugins::{PluginContextType, RequestContext};
use crate::server::NovaServer;
use crate::{
    error::NovaError,
    tools::gecko_terminal::{
        get_networks, get_pool, get_token, GetGeckoNetworksInput, GetGeckoPoolInput,
        GetGeckoTokenInput,
    },
    tools::new_pools::{get_new_pools, GetNewPoolsInput},
    tools::search_pools::{search_pools, SearchPoolsInput},
    tools::trending_pools::{get_trending_pools, GetTrendingPoolsInput},
};
use axum::http::StatusCode;
use serde_json::json;

use super::dto::{McpError, McpRequest, McpResponse, ToolCall, ToolResult};

pub async fn handle_request(
    server: &NovaServer,
    request: McpRequest,
    transport_context: Option<RequestContext>,
) -> McpResponse {
    match request.method.as_str() {
        "tools/list" => match resolve_context(&request, transport_context) {
            Ok(context) => match server.get_tools(&context) {
                Ok(tools) => McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "tools": tools
                    })),
                    error: None,
                },
                Err(err) => error_response(
                    request.id,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to load tools: {}", err),
                ),
            },
            Err(response) => *response,
        },
        "tools/call" => {
            if let Some(params) = request.params.clone() {
                if let Ok(tool_call) = serde_json::from_value::<ToolCall>(params) {
                    match resolve_context(&request, transport_context.clone()) {
                        Ok(context) => match handle_tool_call(server, tool_call, &context).await {
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
                        },
                        Err(response) => *response,
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
    context: &RequestContext,
) -> Result<ToolResult, NovaError> {
    tracing::info!("Handling tool call: {}", tool_call.name);
    let result = match tool_call.name.as_str() {
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
            let (expected_type, expected_id, _base, _version) =
                parse_fully_qualified_name(&tool_call.name)
                    .ok_or_else(|| NovaError::api_error("Invalid tool name"))?;

            let metadata = server
                .plugin_manager()
                .get_plugin_by_fq_name(&tool_call.name)?;

            if metadata.context_type != expected_type || metadata.context_id != expected_id {
                return Err(NovaError::api_error(
                    "Tool context does not match registered owner",
                ));
            }

            let response = server
                .plugin_manager()
                .invoke_plugin(&metadata, context, tool_call.arguments)
                .await?;
            response
        }
    };

    Ok(ToolResult {
        content: serde_json::to_string_pretty(&result)?,
        is_error: false,
    })
}

fn resolve_context(
    request: &McpRequest,
    transport_context: Option<RequestContext>,
) -> Result<RequestContext, Box<McpResponse>> {
    if let Some(context) = transport_context {
        return Ok(context);
    }

    let context_type = request
        .context_type
        .as_ref()
        .map(|value| value.trim().to_lowercase());
    let context_id = request
        .context_id
        .as_ref()
        .map(|value| value.trim().to_string());

    let context_type = match context_type.as_deref() {
        Some("user") => PluginContextType::User,
        Some("group") => PluginContextType::Group,
        _ => {
            return Err(Box::new(error_response(
                request.id.clone(),
                StatusCode::UNAUTHORIZED,
                "Missing or invalid context_type",
            )))
        }
    };

    let context_id = match context_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            return Err(Box::new(error_response(
                request.id.clone(),
                StatusCode::UNAUTHORIZED,
                "Missing or invalid context_id",
            )))
        }
    };

    if context_id.parse::<i64>().is_err() {
        return Err(Box::new(error_response(
            request.id.clone(),
            StatusCode::UNAUTHORIZED,
            "context_id must be numeric",
        )));
    }

    Ok(RequestContext {
        context_type,
        context_id,
    })
}

fn parse_fully_qualified_name(name: &str) -> Option<(PluginContextType, String, String, u32)> {
    if let Some(stripped) = name.strip_prefix("user_") {
        parse_name_parts(stripped)
            .map(|(context_id, base, version)| (PluginContextType::User, context_id, base, version))
    } else if let Some(stripped) = name.strip_prefix("group_") {
        parse_name_parts(stripped).map(|(context_id, base, version)| {
            (PluginContextType::Group, context_id, base, version)
        })
    } else {
        None
    }
}

fn parse_name_parts(input: &str) -> Option<(String, String, u32)> {
    let (context_id, remainder) = input.split_once('_')?;
    let (base, version_part) = remainder.rsplit_once("_v")?;
    let version = version_part.parse::<u32>().ok()?;
    Some((context_id.to_string(), base.to_string(), version))
}

fn error_response(
    id: Option<serde_json::Value>,
    status: StatusCode,
    message: impl Into<String>,
) -> McpResponse {
    McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(McpError {
            code: status.as_u16() as i32,
            message: message.into(),
            data: None,
        }),
    }
}
