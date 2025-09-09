use nova_mcp::{NovaConfig, NovaServer};

#[test]
fn list_tools_has_only_two() {
    let server = NovaServer::new(NovaConfig::default());
    let tools = server.get_tools();
    assert_eq!(tools.len(), 2);
    let names: Vec<_> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"get_cat_fact"));
    assert!(names.contains(&"get_btc_price"));
}

