// Minimal example: call GeckoTerminal tools directly via NovaServer
use anyhow::Result;
use nova_mcp::plugins::PluginManager;
use nova_mcp::server::ToolCall;
use nova_mcp::{NovaConfig, NovaServer};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let server = build_server()?;
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

fn build_server() -> Result<NovaServer> {
    let config = NovaConfig::default();
    let db = sled::Config::new().temporary(true).open()?;
    let user_tree = db.open_tree("user_plugins")?;
    let group_tree = db.open_tree("group_plugins")?;
    let plugin_manager = Arc::new(PluginManager::new(user_tree, group_tree));
    Ok(NovaServer::new(config, plugin_manager))
}
