use nova_mcp::mcp::{dto::McpRequest, handler};
use nova_mcp::{NovaConfig, NovaServer};
use serde_json::json;

#[tokio::test]
async fn invalid_arguments_return_error() {
    let server = NovaServer::new(NovaConfig::default());
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
