// Integration tests that hit real public APIs. Marked ignored by default.
use nova_mcp::server::ToolCall;
use nova_mcp::{NovaConfig, NovaServer};
use serde_json::json;

#[tokio::test]
#[ignore]
async fn get_gecko_networks_live() {
    let server = NovaServer::new(NovaConfig::default());
    let call = ToolCall {
        name: "get_gecko_networks".into(),
        arguments: json!({}),
    };
    let res = server.handle_tool_call(call).await.unwrap();
    assert!(res.content.contains("networks"));
}
