use super::dto::{GetTrendingPoolsInput, GetTrendingPoolsOutput};
use super::implementation::TrendingPoolsTools;
use crate::error::Result;

pub async fn get_trending_pools(
    tools: &TrendingPoolsTools,
    input: GetTrendingPoolsInput,
) -> Result<GetTrendingPoolsOutput> {
    tools.get_trending_pools(input).await
}
