use crate::*;

pub const LOG_TARGET: &str = "neardata-fetcher";

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
pub const DEFAULT_RETRY_DURATION: Duration = Duration::from_secs(1);

pub(crate) const MAINNET_R2_LAST_BLOCK_HEIGHT: u64 = 142000000;
pub(crate) const TESTNET_ARCHIVE_LAST_BLOCK_HEIGHT: u64 = 185670000;
pub(crate) const MAINNET_ARCHIVE_BOUNDARIES: &[u64] = &[122000000, 142000000];

pub(crate) const NUMBER_OF_BLOCKS_PER_ARCHIVE: u64 = 10;
pub(crate) const ARCHIVE_SYNC_THRESHOLD: u64 = NUMBER_OF_BLOCKS_PER_ARCHIVE * 2;

pub fn new_reqwest_client() -> Client {
    Client::new()
}

pub async fn fetch_block(client: &Client, url: &str, timeout: Duration) -> BlockResult {
    let response = client.get(url).timeout(timeout).send().await?;
    Ok(response.json().await?)
}

pub async fn fetch_block_until_success(
    client: &Client,
    url: &str,
    timeout: Duration,
) -> Option<BlockWithTxHashes> {
    loop {
        match fetch_block(client, url, timeout).await {
            Ok(block) => return block,
            Err(FetchError::ReqwestError(err)) => {
                tracing::log::warn!(target: LOG_TARGET, "Failed to fetch block: {}", err);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(FetchError::RedirectError) => {
                tracing::log::warn!(target: LOG_TARGET, "Redirect error");
                return None;
            }
        }
    }
}

pub(crate) fn target_url(suffix: &str, chain_id: ChainId) -> String {
    format!(
        "{}{}",
        match chain_id {
            ChainId::Mainnet => "https://mainnet.neardata.xyz",
            ChainId::Testnet => "https://testnet.neardata.xyz",
        },
        suffix
    )
}

pub async fn fetch_first_block(client: &Client, chain_id: ChainId) -> Option<BlockWithTxHashes> {
    fetch_block_until_success(
        client,
        &target_url("/v0/first_block", chain_id),
        DEFAULT_TIMEOUT,
    )
    .await
}

pub async fn fetch_last_block(client: &Client, chain_id: ChainId) -> Option<BlockWithTxHashes> {
    fetch_block_until_success(
        client,
        &target_url("/v0/last_block/final", chain_id),
        DEFAULT_TIMEOUT,
    )
    .await
}

pub async fn fetch_block_by_height(
    client: &Client,
    height: BlockHeight,
    timeout: Duration,
    chain_id: ChainId,
) -> Option<BlockWithTxHashes> {
    fetch_block_until_success(
        client,
        &target_url(&format!("/v0/block/{}", height), chain_id),
        timeout,
    )
    .await
}
