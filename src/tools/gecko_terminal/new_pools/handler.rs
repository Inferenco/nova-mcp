use super::dto::{GetNewPoolsInput, GetNewPoolsOutput};
use super::implementation::NewPoolsTools;
use crate::error::Result;

pub async fn get_new_pools(
    tools: &NewPoolsTools,
    input: GetNewPoolsInput,
) -> Result<GetNewPoolsOutput> {
    tools.get_new_pools(input).await
}
