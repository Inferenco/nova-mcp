use super::dto::{GetGeckoNetworksInput, GetGeckoNetworksOutput};
use crate::error::Result;
use crate::tools::gecko_terminal::implementation::GeckoTerminalTools;

pub async fn get_networks(
    tools: &GeckoTerminalTools,
    input: GetGeckoNetworksInput,
) -> Result<GetGeckoNetworksOutput> {
    tools.get_networks(input).await
}
