// Integration tests that hit real public APIs. Marked ignored by default.
use nova_mcp::plugins::PluginManager;
use nova_mcp::server::ToolCall;
use nova_mcp::{NovaConfig, NovaServer};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
#[ignore]
async fn get_gecko_networks_live() {
    let server = test_server();
    let call = ToolCall {
        name: "get_gecko_networks".into(),
        arguments: json!({}),
    };
    let res = server.handle_tool_call(call).await.unwrap();
    assert!(res.content.contains("networks"));
}

fn test_server() -> NovaServer {
    let config = NovaConfig::default();
    let db = sled::Config::new().temporary(true).open().unwrap();
    let user_tree = db.open_tree("user_plugins").unwrap();
    let group_tree = db.open_tree("group_plugins").unwrap();
    let plugin_manager = Arc::new(PluginManager::new(user_tree, group_tree));
    NovaServer::new(config, plugin_manager)
}
