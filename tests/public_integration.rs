// Integration tests that hit real public APIs. Marked ignored by default.
use nova_mcp::plugins::{PluginContextType, PluginManager, RequestContext};
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
    let context = RequestContext {
        context_type: PluginContextType::User,
        context_id: "0".to_string(),
    };
    let res = server.handle_tool_call(call, &context).await.unwrap();
    assert!(res.content.contains("networks"));
}

fn test_server() -> NovaServer {
    let config = NovaConfig::default();
    let db = sled::Config::new().temporary(true).open().unwrap();
    let metadata_tree = db.open_tree("plugin_metadata").unwrap();
    let user_tree = db.open_tree("user_plugins").unwrap();
    let group_tree = db.open_tree("group_plugins").unwrap();
    let plugin_manager = Arc::new(
        PluginManager::new(metadata_tree, user_tree, group_tree).expect("init plugin manager"),
    );
    NovaServer::new(config, plugin_manager)
}
