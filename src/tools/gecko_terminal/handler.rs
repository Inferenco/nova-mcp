use super::dto::{
    GetGeckoNetworksInput, GetGeckoNetworksOutput, GetGeckoPoolInput, GetGeckoPoolOutput,
    GetGeckoTokenInput, GetGeckoTokenOutput,
};
use super::implementation::GeckoTerminalTools;
use crate::error::Result;

pub async fn get_networks(
    tools: &GeckoTerminalTools,
    input: GetGeckoNetworksInput,
) -> Result<GetGeckoNetworksOutput> {
    tools.get_networks(input).await
}

pub async fn get_token(
    tools: &GeckoTerminalTools,
    input: GetGeckoTokenInput,
) -> Result<GetGeckoTokenOutput> {
    tools.get_token(input).await
}

pub async fn get_pool(
    tools: &GeckoTerminalTools,
    input: GetGeckoPoolInput,
) -> Result<GetGeckoPoolOutput> {
    tools.get_pool(input).await
}
