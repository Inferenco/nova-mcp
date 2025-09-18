use axum::http::HeaderMap;
use serde_json::Value;

use crate::error::NovaError;
use crate::plugins::PluginContextType;

use super::dto::RequestContext;

const HEADER_CONTEXT_TYPE: &str = "x-nova-context-type";
const HEADER_CONTEXT_ID: &str = "x-nova-context-id";

pub fn extract_context_from_headers(
    headers: &HeaderMap,
) -> Result<Option<RequestContext>, NovaError> {
    let context_type = headers
        .get(HEADER_CONTEXT_TYPE)
        .map(|value| value.to_str().map(|s| s.trim().to_string()));
    let context_id = headers
        .get(HEADER_CONTEXT_ID)
        .map(|value| value.to_str().map(|s| s.trim().to_string()));

    match (context_type, context_id) {
        (None, None) => Ok(None),
        (Some(Ok(context_type)), Some(Ok(context_id))) => {
            let parsed_type = parse_context_type(&context_type)?;
            validate_context_pair(&parsed_type, &context_id)?;
            Ok(Some(RequestContext {
                context_type: parsed_type,
                context_id,
            }))
        }
        (Some(Err(_)), _) | (_, Some(Err(_))) => Err(NovaError::validation_error(
            "Invalid UTF-8 in context headers",
        )),
        _ => Err(NovaError::validation_error(
            "Both x-nova-context-type and x-nova-context-id are required",
        )),
    }
}

pub fn extract_context_from_value(value: &Value) -> Result<Option<RequestContext>, NovaError> {
    let context_type = value.get("context_type");
    let context_id = value.get("context_id");

    match (context_type, context_id) {
        (None, None) => Ok(None),
        (Some(Value::String(context_type)), Some(Value::String(context_id))) => {
            let parsed_type = parse_context_type(context_type)?;
            validate_context_pair(&parsed_type, context_id)?;
            Ok(Some(RequestContext {
                context_type: parsed_type,
                context_id: context_id.clone(),
            }))
        }
        _ => Err(NovaError::validation_error(
            "context_type and context_id must be strings when provided",
        )),
    }
}

pub fn require_matching_context(
    expected: &RequestContext,
    supplied: &RequestContext,
) -> Result<(), NovaError> {
    if expected.context_type != supplied.context_type || expected.context_id != supplied.context_id
    {
        Err(NovaError::validation_error(
            "Context mismatch between caller and tool",
        ))
    } else {
        Ok(())
    }
}

pub fn context_key_for_rate_limit(context: Option<&RequestContext>, fallback_key: &str) -> String {
    match context {
        Some(ctx) => match ctx.context_type {
            PluginContextType::User => format!("user:{}", ctx.context_id),
            PluginContextType::Group => format!("group:{}", ctx.context_id),
        },
        None => format!("api:{}", fallback_key),
    }
}

pub fn parse_context_type(value: &str) -> Result<PluginContextType, NovaError> {
    match value.to_lowercase().as_str() {
        "user" => Ok(PluginContextType::User),
        "group" => Ok(PluginContextType::Group),
        other => Err(NovaError::validation_error(format!(
            "Unknown context type: {}",
            other
        ))),
    }
}

pub fn validate_context_pair(
    context_type: &PluginContextType,
    context_id: &str,
) -> Result<(), NovaError> {
    if context_id.trim().is_empty() {
        return Err(NovaError::validation_error("context_id cannot be empty"));
    }

    if context_id.parse::<i64>().is_err() {
        return Err(NovaError::validation_error(
            "context_id must be a numeric string",
        ));
    }

    match context_type {
        PluginContextType::User => {
            if context_id.starts_with('-') {
                return Err(NovaError::validation_error(
                    "User context IDs must be positive",
                ));
            }
        }
        PluginContextType::Group => {
            if !context_id.starts_with('-') {
                return Err(NovaError::validation_error(
                    "Group context IDs must be negative",
                ));
            }
        }
    }

    Ok(())
}
