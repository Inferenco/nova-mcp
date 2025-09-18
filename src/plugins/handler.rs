use axum::{
    extract::{Path, State},
    http::HeaderMap,
    http::StatusCode,
    Json,
};

use crate::http::AppState;

use super::dto::{
    ErrorResponse, PluginEnableRequest, PluginEnablementStatus, PluginInvocationRequest,
    PluginMetadata, PluginRegistrationRequest, PluginUpdateRequest, RequestContext,
};
use super::helpers::{authorize_request, map_error};

pub(crate) async fn register_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PluginRegistrationRequest>,
) -> Result<(StatusCode, Json<PluginMetadata>), (StatusCode, Json<ErrorResponse>)> {
    let context = authorize_request(&state, &headers).await?;
    match state.plugin_manager().register_plugin(&context, request) {
        Ok(metadata) => Ok((StatusCode::CREATED, Json(metadata))),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn unregister_plugin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(plugin_id): Path<u64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let context = authorize_request(&state, &headers).await?;
    match state
        .plugin_manager()
        .unregister_plugin(&context, plugin_id)
    {
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
    let context = authorize_request(&state, &headers).await?;
    match state
        .plugin_manager()
        .update_plugin(&context, plugin_id, request)
    {
        Ok(metadata) => Ok(Json(metadata)),
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn list_plugins(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PluginMetadata>>, (StatusCode, Json<ErrorResponse>)> {
    let context = authorize_request(&state, &headers).await?;
    match state.plugin_manager().list_plugins_for_context(&context) {
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
    let context = authorize_request(&state, &headers).await?;
    let manager = state.plugin_manager_arc();
    match manager.get_plugin(plugin_id) {
        Ok(metadata) => match manager
            .invoke_plugin(&metadata, &context, request.arguments)
            .await
        {
            Ok(value) => Ok(Json(value)),
            Err(err) => Err(map_error(err)),
        },
        Err(err) => Err(map_error(err)),
    }
}

pub(crate) async fn set_plugin_enablement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PluginEnableRequest>,
) -> Result<Json<PluginEnablementStatus>, (StatusCode, Json<ErrorResponse>)> {
    let _context: RequestContext = authorize_request(&state, &headers).await?;
    match state.plugin_manager().set_enablement(request) {
        Ok(status) => Ok(Json(status)),
        Err(err) => Err(map_error(err)),
    }
}
