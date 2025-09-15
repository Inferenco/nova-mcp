use nova_mcp::mcp::{dto::McpRequest, handler};
use nova_mcp::plugins::PluginManager;
use nova_mcp::{NovaConfig, NovaServer};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn invalid_arguments_return_error() {
    let server = test_server();
    let req = McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "get_gecko_networks",
            "arguments": "nope"
        })),
    };
    let resp = handler::handle_request(&server, req).await;
    assert!(resp.result.is_none());
    if let Some(err) = resp.error {
        assert_eq!(err.code, -32603);
    } else {
        panic!("expected error response");
    }
}

fn test_server() -> NovaServer {
    let config = NovaConfig::default();
    let db = sled::Config::new().temporary(true).open().unwrap();
    let user_tree = db.open_tree("user_plugins").unwrap();
    let group_tree = db.open_tree("group_plugins").unwrap();
    let plugin_manager = Arc::new(PluginManager::new(user_tree, group_tree));
    NovaServer::new(config, plugin_manager)
}
