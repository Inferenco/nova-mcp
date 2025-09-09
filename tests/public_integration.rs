// Integration tests that hit real public APIs. Marked ignored by default.
use nova_mcp::{NovaConfig, NovaServer};
use nova_mcp::server::ToolCall;
use serde_json::json;

#[tokio::test]
#[ignore]
async fn get_cat_fact_live() {
    let server = NovaServer::new(NovaConfig::default());
    let call = ToolCall { name: "get_cat_fact".into(), arguments: json!({}) };
    let res = server.handle_tool_call(call).await.unwrap();
    assert!(res.content.contains("fact") || res.content.contains("Fact"));
}

#[tokio::test]
#[ignore]
async fn get_btc_price_live() {
    let server = NovaServer::new(NovaConfig::default());
    let call = ToolCall { name: "get_btc_price".into(), arguments: json!({}) };
    let res = server.handle_tool_call(call).await.unwrap();
    assert!(res.content.contains("usd_price"));
}

