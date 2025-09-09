use crate::server::{McpRequest, McpResponse};
use crate::NovaServer;
use anyhow::Result;
use axum::{routing::post, Json, Router};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    server: Arc<NovaServer>,
}

async fn handle_rpc(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<McpRequest>,
) -> Json<McpResponse> {
    let res = state.server.handle_request(req).await;
    Json(res)
}

pub async fn run_http_server(server: NovaServer, port: u16) -> Result<()> {
    let state = AppState { server: Arc::new(server) };

    let app = Router::new()
        .route("/rpc", post(handle_rpc))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting HTTP MCP server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

