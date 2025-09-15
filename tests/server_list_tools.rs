use nova_mcp::{NovaConfig, NovaServer};

#[test]
fn list_tools_contains_expected() {
    let server = NovaServer::new(NovaConfig::default());
    let tools = server.get_tools();
    assert_eq!(tools.len(), 6);
    let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"get_gecko_networks"));
    assert!(names.contains(&"get_gecko_token"));
    assert!(names.contains(&"get_gecko_pool"));
    assert!(names.contains(&"get_trending_pools"));
    assert!(names.contains(&"search_pools"));
    assert!(names.contains(&"get_new_pools"));
}
