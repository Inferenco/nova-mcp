// Minimal example: call GeckoTerminal tools directly via NovaServer
use anyhow::Result;
use nova_mcp::server::ToolCall;
use nova_mcp::{NovaConfig, NovaServer};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let server = NovaServer::new(NovaConfig::default());
    println!("Available tools:");
    for t in server.get_tools() {
        println!(" - {}: {}", t.name, t.description);
    }

    let networks = ToolCall {
        name: "get_gecko_networks".into(),
        arguments: json!({}),
    };
    println!(
        "gecko_networks -> {:?}",
        server.handle_tool_call(networks).await?.content
    );

    let trending = ToolCall {
        name: "get_trending_pools".into(),
        arguments: json!({"network": "eth", "limit": 5}),
    };
    println!(
        "trending_pools -> {:?}",
        server.handle_tool_call(trending).await?.content
    );
    Ok(())
}
