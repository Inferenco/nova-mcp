use super::dto::{GetGeckoPoolInput, GetGeckoPoolOutput};
use crate::error::Result;
use crate::tools::gecko_terminal::implementation::GeckoTerminalTools;

pub async fn get_pool(
    tools: &GeckoTerminalTools,
    input: GetGeckoPoolInput,
) -> Result<GetGeckoPoolOutput> {
    tools.get_pool(input).await
}
