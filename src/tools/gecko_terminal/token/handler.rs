use super::dto::{GetGeckoTokenInput, GetGeckoTokenOutput};
use crate::error::Result;
use crate::tools::gecko_terminal::implementation::GeckoTerminalTools;

pub async fn get_token(
    tools: &GeckoTerminalTools,
    input: GetGeckoTokenInput,
) -> Result<GetGeckoTokenOutput> {
    tools.get_token(input).await
}
