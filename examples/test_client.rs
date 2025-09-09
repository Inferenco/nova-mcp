// Minimal example: call the two public tools directly via NovaServer
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

    let cat = ToolCall {
        name: "get_cat_fact".into(),
        arguments: json!({}),
    };
    println!(
        "cat_fact -> {:?}",
        server.handle_tool_call(cat).await?.content
    );

    let btc = ToolCall {
        name: "get_btc_price".into(),
        arguments: json!({}),
    };
    println!(
        "btc_price -> {:?}",
        server.handle_tool_call(btc).await?.content
    );
    Ok(())
}
