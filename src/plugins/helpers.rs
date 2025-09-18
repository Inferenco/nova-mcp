use axum::{
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::context::{context_key_for_rate_limit, extract_context_from_headers, RequestContext};
use crate::error::NovaError;
use crate::http::{check_rate_limit, AppState};

use super::dto::ErrorResponse;

pub(crate) async fn authorize_request(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(String, Option<RequestContext>), (StatusCode, Json<ErrorResponse>)> {
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

    let api_key = presented.unwrap_or("anonymous").to_string();
    let context = match extract_context_from_headers(headers) {
        Ok(context) => context,
        Err(err) => {
            let body = ErrorResponse {
                error: err.to_string(),
                details: None,
            };
            return Err((StatusCode::BAD_REQUEST, Json(body)));
        }
    };

    let rate_key = context_key_for_rate_limit(context.as_ref(), &api_key);

    if let Some(code) = check_rate_limit(state, &rate_key).await {
        let body = ErrorResponse {
            error: "Rate limit exceeded".to_string(),
            details: None,
        };
        return Err((code, Json(body)));
    }

    Ok((api_key, context))
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
