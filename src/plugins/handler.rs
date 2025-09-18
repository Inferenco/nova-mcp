use axum::{
    extract::{Path, State},
    http::HeaderMap,
    http::StatusCode,
    Json,
};

use crate::context::RequestContext;
use crate::http::AppState;

use super::dto::{
    ErrorResponse, PluginEnableRequest, PluginEnablementStatus, PluginInvocationRequest,
    PluginMetadata, PluginRegistrationRequest, PluginUpdateRequest, ToolRegistrationResponse,
    ToolUpdateRequest,
};
use super::helpers::{authorize_request, map_error};

pub(crate) async fn register_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PluginRegistrationRequest>,
) -> Result<(StatusCode, Json<PluginMetadata>), (StatusCode, Json<ErrorResponse>)> {
    let _ = authorize_request(&state, &headers).await?;
    match state.plugin_manager().register_plugin(request) {
        Ok(metadata) => Ok((StatusCode::CREATED, Json(metadata))),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn unregister_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plugin_id): Path<u64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _ = authorize_request(&state, &headers).await?;
    match state.plugin_manager().unregister_plugin(plugin_id) {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn update_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plugin_id): Path<u64>,
    Json(request): Json<PluginUpdateRequest>,
) -> Result<Json<PluginMetadata>, (StatusCode, Json<ErrorResponse>)> {
    let _ = authorize_request(&state, &headers).await?;
    match state.plugin_manager().update_plugin(plugin_id, request) {
        Ok(metadata) => Ok(Json(metadata)),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn list_plugins(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PluginMetadata>>, (StatusCode, Json<ErrorResponse>)> {
    let _ = authorize_request(&state, &headers).await?;
    match state.plugin_manager().list_plugins() {
        Ok(list) => Ok(Json(list)),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn invoke_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plugin_id): Path<u64>,
    Json(request): Json<PluginInvocationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let _ = authorize_request(&state, &headers).await?;
    let manager = state.plugin_manager_arc();
    match manager.invoke_plugin(plugin_id, request).await {
        Ok(value) => Ok(Json(value)),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn set_plugin_enablement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PluginEnableRequest>,
) -> Result<Json<PluginEnablementStatus>, (StatusCode, Json<ErrorResponse>)> {
    let _ = authorize_request(&state, &headers).await?;
    match state.plugin_manager().set_enablement(request) {
        Ok(status) => Ok(Json(status)),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn register_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<PluginRegistrationRequest>,
) -> Result<(StatusCode, Json<ToolRegistrationResponse>), (StatusCode, Json<ErrorResponse>)> {
    let (_, context) = authorize_request(&state, &headers).await?;
    let owner_context = require_context(&context)?;
    request.context_type = Some(owner_context.context_type.clone());
    request.context_id = Some(owner_context.context_id.clone());
    match state
        .plugin_manager()
        .register_tool(request, &owner_context)
    {
        Ok(response) => Ok((StatusCode::CREATED, Json(response))),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn list_tools(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PluginMetadata>>, (StatusCode, Json<ErrorResponse>)> {
    let (_, context) = authorize_request(&state, &headers).await?;
    let owner_context = require_context(&context)?;
    match state.plugin_manager().list_plugins_for_context(
        owner_context.context_type.clone(),
        &owner_context.context_id,
    ) {
        Ok(list) => Ok(Json(list)),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn update_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plugin_id): Path<u64>,
    Json(request): Json<ToolUpdateRequest>,
) -> Result<Json<PluginMetadata>, (StatusCode, Json<ErrorResponse>)> {
    let (_, context) = authorize_request(&state, &headers).await?;
    let owner_context = require_context(&context)?;
    match state
        .plugin_manager()
        .update_tool(plugin_id, request, &owner_context)
    {
        Ok(metadata) => Ok(Json(metadata)),
        Err(err) => Err(map_error(err)),
    }
}

fn require_context(
    context: &Option<RequestContext>,
) -> Result<RequestContext, (StatusCode, Json<ErrorResponse>)> {
    match context {
        Some(ctx) => Ok(ctx.clone()),
        None => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Context headers are required".to_string(),
                details: None,
            }),
        )),
    }
}
