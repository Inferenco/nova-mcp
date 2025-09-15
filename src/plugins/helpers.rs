use axum::{
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::error::NovaError;
use crate::http::{check_rate_limit, AppState};

use super::dto::ErrorResponse;

pub(crate) async fn authorize_request(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
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

    let key = presented.unwrap_or("anonymous").to_string();
    if let Some(code) = check_rate_limit(state, &key).await {
        let body = ErrorResponse {
            error: "Rate limit exceeded".to_string(),
            details: None,
        };
        return Err((code, Json(body)));
    }

    Ok(key)
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
