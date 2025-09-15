pub mod gecko_terminal;

pub use gecko_terminal::{
    get_networks, get_pool, get_token, GeckoTerminalTools, GetGeckoNetworksInput,
    GetGeckoNetworksOutput, GetGeckoPoolInput, GetGeckoPoolOutput, GetGeckoTokenInput,
    GetGeckoTokenOutput,
};
// Re-export submodules so existing imports like `tools::new_pools::...` continue to work
pub use gecko_terminal::new_pools;
pub use gecko_terminal::search_pools;
pub use gecko_terminal::trending_pools;

// And also re-export common types/functions at the root for convenience
pub use gecko_terminal::new_pools::{
    get_new_pools, GetNewPoolsInput, GetNewPoolsOutput, NewPoolsTools,
};
pub use gecko_terminal::search_pools::{SearchPoolsInput, SearchPoolsOutput, SearchPoolsTools};
pub use gecko_terminal::trending_pools::{
    get_trending_pools, GetTrendingPoolsInput, GetTrendingPoolsOutput, TrendingPoolsTools,
};
