use super::dto::{SearchPoolsInput, SearchPoolsOutput};
use super::implementation::SearchPoolsTools;
use crate::error::Result;

pub async fn search_pools(
    tools: &SearchPoolsTools,
    input: SearchPoolsInput,
) -> Result<SearchPoolsOutput> {
    tools.search_pools(input).await
}
