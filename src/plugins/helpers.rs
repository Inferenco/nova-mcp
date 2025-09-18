use axum::{
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::error::NovaError;
use crate::http::{check_rate_limit, AppState};

use super::dto::{ErrorResponse, PluginContextType, RequestContext};

const CONTEXT_TYPE_HEADER: &str = "x-nova-context-type";
const CONTEXT_ID_HEADER: &str = "x-nova-context-id";

pub(crate) async fn authorize_request(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<RequestContext, (StatusCode, Json<ErrorResponse>)> {
    let header_name = state.auth().header_name().to_string();
    let presented = headers
        .get(header_name.as_str())
        .and_then(|value| value.to_str().ok());

    if !state.auth().validate(presented) {
        let body = ErrorResponse {
            error: "Unauthorized".to_string(),
            details: None,
        };
        return Err((StatusCode::UNAUTHORIZED, Json(body)));
    }

    let context_type = headers
        .get(CONTEXT_TYPE_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_lowercase());

    let context_type = match context_type.as_deref() {
        Some("user") => PluginContextType::User,
        Some("group") => PluginContextType::Group,
        _ => {
            let body = ErrorResponse {
                error: "Invalid or missing x-nova-context-type".to_string(),
                details: None,
            };
            return Err((StatusCode::BAD_REQUEST, Json(body)));
        }
    };

    let context_id_value = headers
        .get(CONTEXT_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_string());

    let context_id = match context_id_value {
        Some(ref id) if !id.is_empty() => id.clone(),
        _ => {
            let body = ErrorResponse {
                error: "Invalid or missing x-nova-context-id".to_string(),
                details: None,
            };
            return Err((StatusCode::BAD_REQUEST, Json(body)));
        }
    };

    if context_id.parse::<i64>().is_err() {
        let body = ErrorResponse {
            error: "x-nova-context-id must be a numeric identifier".to_string(),
            details: None,
        };
        return Err((StatusCode::BAD_REQUEST, Json(body)));
    }

    let context = RequestContext {
        context_type,
        context_id,
    };

    let rate_key = format!(
        "{}:{}",
        match context.context_type {
            PluginContextType::User => "user",
            PluginContextType::Group => "group",
        },
        context.context_id
    );

    if let Some(code) = check_rate_limit(state, &rate_key).await {
        let body = ErrorResponse {
            error: "Rate limit exceeded".to_string(),
            details: None,
        };
        return Err((code, Json(body)));
    }

    Ok(context)
}

pub(crate) fn map_error(err: NovaError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, details) = match &err {
        NovaError::PluginNotFound { .. } => (StatusCode::NOT_FOUND, None),
        NovaError::PluginNotEnabled { .. } => (StatusCode::FORBIDDEN, None),
        NovaError::ValidationError { .. } => (StatusCode::BAD_REQUEST, None),
        NovaError::RateLimitExceeded { .. } => (StatusCode::TOO_MANY_REQUESTS, None),
        NovaError::ApiError(_) | NovaError::NetworkError(_) => (StatusCode::BAD_GATEWAY, None),
        NovaError::StorageError(_) => (StatusCode::SERVICE_UNAVAILABLE, None),
        NovaError::SerializationError(_) => (StatusCode::INTERNAL_SERVER_ERROR, None),
        NovaError::ConfigError(_) => (StatusCode::BAD_REQUEST, None),
        NovaError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, None),
        NovaError::PoolNotFound { .. }
        | NovaError::TokenNotFound { .. }
        | NovaError::InvalidAddress { .. } => (StatusCode::BAD_REQUEST, None),
    };

    let body = ErrorResponse {
        error: err.to_string(),
        details,
    };

    (status, Json(body))
}
