use crate::mcp::dto::{McpRequest, McpResponse};
use crate::plugins::{self, PluginManager};
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
        let res = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: None,
            error: Some(crate::mcp::dto::McpError {
                code: StatusCode::UNAUTHORIZED.as_u16() as i32,
                message: "Unauthorized".to_string(),
                data: None,
            }),
        };
        return Json(res);
    }
    // Simple per-key rate limiting
    let key = presented.unwrap_or("anonymous").to_string();
    if let Some(code) = check_rate_limit(&state, &key).await {
        let res = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: None,
            error: Some(crate::mcp::dto::McpError {
                code: code.as_u16() as i32,
                message: "Rate limit exceeded".to_string(),
                data: None,
            }),
        };
        return Json(res);
    }
    let server = state.server();
    let res = crate::mcp::handler::handle_request(server.as_ref(), req).await;
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
