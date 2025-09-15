use nova_mcp::tools::{
    new_pools::{GetNewPoolsInput, NewPoolsTools},
    search_pools::{SearchPoolsInput, SearchPoolsTools},
    trending_pools::{GetTrendingPoolsInput, TrendingPoolsTools},
};

#[tokio::test]
async fn trending_pools_invalid_limit() {
    let tools = TrendingPoolsTools::new();
    let input = GetTrendingPoolsInput {
        network: "eth".to_string(),
        limit: Some(21),
        page: None,
        duration: None,
    };
    let result = tools.get_trending_pools(input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn search_pools_empty_query() {
    let tools = SearchPoolsTools::new();
    let input = SearchPoolsInput {
        query: "".to_string(),
        network: None,
        page: None,
    };
    let result = tools.search_pools(input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn new_pools_invalid_page() {
    let tools = NewPoolsTools::new();
    let input = GetNewPoolsInput {
        network: "eth".to_string(),
        page: Some(0),
    };
    let result = tools.get_new_pools(input).await;
    assert!(result.is_err());
}
