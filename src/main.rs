use anyhow::Result;
use nova_mcp::{NovaConfig, NovaServer};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nova_mcp=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load .env for local dev (if present)
    if dotenvy::dotenv().is_ok() {
        tracing::info!("Loaded .env");
    }

    tracing::info!("Starting Nova MCP Server");

    // Load configuration
    let config = NovaConfig::from_env()?;
    tracing::info!(
        "Configuration loaded: transport={}, port={}",
        config.server.transport,
        config.server.port
    );

    // Create server instance
    let server = NovaServer::new(config.clone());

    tracing::info!("Available tools: {}", server.get_tools().len());
    for tool in server.get_tools() {
        tracing::info!("  - {}: {}", tool.name, tool.description);
    }

    match config.server.transport.to_lowercase().as_str() {
        "http" => {
            tracing::info!(
                "Nova MCP Server running with HTTP transport on port {}",
                config.server.port
            );
            nova_mcp::http::run_http_server(server, config.clone()).await?;
            Ok(())
        }
        _ => {
            tracing::info!("Nova MCP Server running with stdio transport");

            // Handle stdio MCP protocol
            let stdin = io::stdin();
            let mut stdout = io::stdout();
            let mut reader = BufReader::new(stdin);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        tracing::debug!("Received: {}", line);

                        match serde_json::from_str::<nova_mcp::mcp::dto::McpRequest>(line) {
                            Ok(request) => {
                                let response =
                                    nova_mcp::mcp::handler::handle_request(&server, request).await;
                                let response_json = serde_json::to_string(&response)?;

                                tracing::debug!("Sending: {}", response_json);

                                stdout.write_all(response_json.as_bytes()).await?;
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                            }
                            Err(e) => {
                                tracing::error!("Failed to parse request: {}", e);
                                let error_response = nova_mcp::mcp::dto::McpResponse {
                                    jsonrpc: "2.0".to_string(),
                                    id: None,
                                    result: None,
                                    error: Some(nova_mcp::mcp::dto::McpError {
                                        code: -32700,
                                        message: "Parse error".to_string(),
                                        data: Some(serde_json::json!({"details": e.to_string()})),
                                    }),
                                };

                                let error_json = serde_json::to_string(&error_response)?;
                                stdout.write_all(error_json.as_bytes()).await?;
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from stdin: {}", e);
                        break;
                    }
                }
            }

            tracing::info!("Nova MCP Server shutting down");
            Ok(())
        }
    }
}
