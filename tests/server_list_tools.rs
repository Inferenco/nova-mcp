use nova_mcp::plugins::{PluginContextType, PluginManager, RequestContext};
use nova_mcp::{NovaConfig, NovaServer};
use std::sync::Arc;

#[test]
fn list_tools_contains_expected() {
    let server = test_server();
    let context = RequestContext {
        context_type: PluginContextType::User,
        context_id: "0".to_string(),
    };
    let tools = server.get_tools(&context).unwrap();
    assert_eq!(tools.len(), 6);
    let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"get_gecko_networks"));
    assert!(names.contains(&"get_gecko_token"));
    assert!(names.contains(&"get_gecko_pool"));
    assert!(names.contains(&"get_trending_pools"));
    assert!(names.contains(&"search_pools"));
    assert!(names.contains(&"get_new_pools"));
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
