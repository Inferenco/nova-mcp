use crate::context::{
    parse_context_type, require_matching_context, validate_context_pair, RequestContext,
};
use crate::plugins::{PluginInvocationRequest, PluginManager};
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
use serde_json::json;

use super::dto::{McpError, McpRequest, McpResponse, ToolCall, ToolResult};

pub async fn handle_request(
    server: &NovaServer,
    request: McpRequest,
    context_override: Option<RequestContext>,
) -> McpResponse {
    let context = match resolve_context(
        request.context_type.as_deref(),
        request.context_id.as_deref(),
        context_override,
    ) {
        Ok(ctx) => ctx,
        Err(err) => {
            return McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32602,
                    message: err.to_string(),
                    data: None,
                }),
            };
        }
    };

    match request.method.as_str() {
        "tools/list" => {
            let tools = server.get_tools(context.as_ref());
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
                    match handle_tool_call(server, tool_call, context.as_ref()).await {
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
                    invalid_params(request.id, "Invalid tool call parameters")
                }
            } else {
                invalid_params(request.id, "Missing parameters")
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
    context: Option<&RequestContext>,
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
        _ => handle_plugin_tool_call(server.plugin_manager(), tool_call, context).await?,
    };

    Ok(ToolResult {
        content: serde_json::to_string_pretty(&result)?,
        is_error: false,
    })
}

async fn handle_plugin_tool_call(
    manager: &PluginManager,
    tool_call: ToolCall,
    context: Option<&RequestContext>,
) -> Result<serde_json::Value, NovaError> {
    let caller_context = context.ok_or_else(|| {
        NovaError::validation_error("Context headers are required for custom tools")
    })?;

    let metadata = manager.get_plugin_by_fq_name(&tool_call.name)?;

    let plugin_context_type = metadata
        .context_type
        .clone()
        .ok_or_else(|| NovaError::validation_error("Plugin missing context type"))?;
    let plugin_context_id = metadata
        .context_id
        .clone()
        .ok_or_else(|| NovaError::validation_error("Plugin missing context id"))?;

    let plugin_context = RequestContext {
        context_type: plugin_context_type,
        context_id: plugin_context_id,
    };

    require_matching_context(&plugin_context, caller_context)?;

    let invocation = PluginInvocationRequest {
        context_type: caller_context.context_type.clone(),
        context_id: caller_context.context_id.clone(),
        arguments: tool_call.arguments,
    };

    manager.invoke_plugin(metadata.plugin_id, invocation).await
}

fn invalid_params(id: Option<serde_json::Value>, message: &str) -> McpResponse {
    McpResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(McpError {
            code: -32602,
            message: message.to_string(),
            data: None,
        }),
    }
}

fn resolve_context(
    context_type: Option<&str>,
    context_id: Option<&str>,
    override_context: Option<RequestContext>,
) -> Result<Option<RequestContext>, NovaError> {
    if override_context.is_some() {
        return Ok(override_context);
    }

    match (context_type, context_id) {
        (Some(context_type), Some(context_id)) => {
            let parsed = parse_context_type(context_type)?;
            validate_context_pair(&parsed, context_id)?;
            Ok(Some(RequestContext {
                context_type: parsed,
                context_id: context_id.to_string(),
            }))
        }
        (None, None) => Ok(None),
        _ => Err(NovaError::validation_error(
            "Both context_type and context_id are required",
        )),
    }
}
