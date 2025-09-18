use nova_mcp::plugins::PluginManager;
use nova_mcp::{NovaConfig, NovaServer};
use std::sync::Arc;

#[test]
fn list_tools_contains_expected() {
    let server = test_server();
    let tools = server.get_tools(None);
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
    let user_tree = db.open_tree("user_plugins").unwrap();
    let group_tree = db.open_tree("group_plugins").unwrap();
    let plugin_manager = Arc::new(PluginManager::new(user_tree, group_tree));
    NovaServer::new(config, plugin_manager)
}
