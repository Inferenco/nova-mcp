use crate::mcp::dto::{McpError, McpRequest, McpResponse};
use crate::plugins::{self, PluginContextType, PluginManager, RequestContext};
use crate::{ApiKeyAuth, NovaConfig, NovaServer};
use anyhow::Result;
use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

#[derive(Clone)]
pub(crate) struct AppState {
    server: Arc<NovaServer>,
    plugin_manager: Arc<PluginManager>,
    auth: ApiKeyAuth,
    rate: Arc<Mutex<HashMap<String, RateState>>>,
    limit_per_minute: u32,
    ttl_seconds: u64,
}

impl AppState {
    pub(crate) fn server(&self) -> Arc<NovaServer> {
        Arc::clone(&self.server)
    }

    pub(crate) fn plugin_manager(&self) -> &PluginManager {
        self.plugin_manager.as_ref()
    }

    pub(crate) fn plugin_manager_arc(&self) -> Arc<PluginManager> {
        Arc::clone(&self.plugin_manager)
    }

    pub(crate) fn auth(&self) -> &ApiKeyAuth {
        &self.auth
    }
}

async fn handle_rpc(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<McpRequest>,
) -> Json<McpResponse> {
    // API key enforcement
    let header_name = state.auth().header_name().to_string();
    let presented = headers
        .get(header_name.as_str())
        .and_then(|v| v.to_str().ok());
    if !state.auth().validate(presented) {
        let res = rpc_error_response(None, StatusCode::UNAUTHORIZED, "Unauthorized");
        return Json(res);
    }

    let context = match extract_context_from_headers(&headers, req.id.clone()) {
        Ok(context) => context,
        Err(response) => return Json(*response),
    };

    let rate_key = format!(
        "{}:{}",
        match context.context_type {
            PluginContextType::User => "user",
            PluginContextType::Group => "group",
        },
        context.context_id
    );

    if let Some(code) = check_rate_limit(&state, &rate_key).await {
        let res = rpc_error_response(req.id.clone(), code, "Rate limit exceeded");
        return Json(res);
    }

    let server = state.server();
    let res = crate::mcp::handler::handle_request(server.as_ref(), req, Some(context)).await;
    Json(res)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz() -> &'static str {
    "ready"
}

pub async fn run_http_server(server: NovaServer, config: NovaConfig) -> Result<()> {
    let plugin_manager = server.plugin_manager_arc();
    let state = AppState {
        server: Arc::new(server),
        plugin_manager,
        auth: crate::ApiKeyAuth::new(&config.auth),
        rate: Arc::new(Mutex::new(HashMap::new())),
        limit_per_minute: config.apis.rate_limit_per_minute,
        ttl_seconds: config.cache.ttl_seconds,
    };

    let app = Router::new()
        .route("/rpc", post(handle_rpc))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/plugins/register", post(plugins::register_plugin))
        .route(
            "/plugins/:plugin_id",
            delete(plugins::unregister_plugin).put(plugins::update_plugin),
        )
        .route("/plugins", get(plugins::list_plugins))
        .route("/plugins/:plugin_id/call", post(plugins::invoke_plugin))
        .route("/plugins/enable", post(plugins::set_plugin_enablement))
        .route("/tools/register", post(plugins::register_plugin))
        .route(
            "/tools/:plugin_id",
            delete(plugins::unregister_plugin).put(plugins::update_plugin),
        )
        .route("/tools", get(plugins::list_plugins))
        .route("/tools/:plugin_id/call", post(plugins::invoke_plugin))
        .route("/tools/enable", post(plugins::set_plugin_enablement))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Starting HTTP MCP server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("HTTP server error: {}", e);
    }
    Ok(())
}

fn extract_context_from_headers(
    headers: &axum::http::HeaderMap,
    id: Option<serde_json::Value>,
) -> Result<RequestContext, Box<McpResponse>> {
    let context_type = headers
        .get("x-nova-context-type")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_lowercase());

    let context_type = match context_type.as_deref() {
        Some("user") => PluginContextType::User,
        Some("group") => PluginContextType::Group,
        _ => {
            return Err(Box::new(rpc_error_response(
                id,
                StatusCode::BAD_REQUEST,
                "Invalid or missing x-nova-context-type",
            )))
        }
    };

    let context_id_value = headers
        .get("x-nova-context-id")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_string());

    let context_id = match context_id_value {
        Some(ref value) if !value.is_empty() => value.clone(),
        _ => {
            return Err(Box::new(rpc_error_response(
                id,
                StatusCode::BAD_REQUEST,
                "Invalid or missing x-nova-context-id",
            )))
        }
    };

    if context_id.parse::<i64>().is_err() {
        return Err(Box::new(rpc_error_response(
            id,
            StatusCode::BAD_REQUEST,
            "x-nova-context-id must be numeric",
        )));
    }

    Ok(RequestContext {
        context_type,
        context_id,
    })
}

fn rpc_error_response(
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

#[derive(Clone, Debug)]
struct RateState {
    window_start_sec: u64,
    count: u32,
    last_seen_sec: u64,
}

pub(crate) async fn check_rate_limit(state: &AppState, key: &str) -> Option<StatusCode> {
    let now_sec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs();
    let minute_bucket = now_sec / 60;
    let mut map = state.rate.lock().await;
    map.retain(|_, v| now_sec.saturating_sub(v.last_seen_sec) <= state.ttl_seconds);
    let entry = map.entry(key.to_string()).or_insert(RateState {
        window_start_sec: minute_bucket,
        count: 0,
        last_seen_sec: now_sec,
    });
    if entry.window_start_sec != minute_bucket {
        entry.window_start_sec = minute_bucket;
        entry.count = 0;
    }
    entry.last_seen_sec = now_sec;
    if entry.count >= state.limit_per_minute {
        Some(StatusCode::TOO_MANY_REQUESTS)
    } else {
        entry.count += 1;
        None
    }
}
