use fastnear_primitives::block_with_tx_hash::BlockWithTxHashes;
use fastnear_primitives::near_primitives::types::BlockHeight;
use fastnear_primitives::types::ChainId;
use reqwest::Client;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

const LOG_TARGET: &str = "neardata-fetcher";

pub type BlockResult = Result<Option<BlockWithTxHashes>, FetchError>;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum FetchError {
    ReqwestError(reqwest::Error),
}

impl From<reqwest::Error> for FetchError {
    fn from(error: reqwest::Error) -> Self {
        FetchError::ReqwestError(error)
    }
}

#[derive(Debug, Clone)]
pub struct FetcherConfig {
    pub num_threads: u64,
    pub start_block_height: BlockHeight,
    pub chain_id: ChainId,
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
        }
    }
}

fn target_url(suffix: &str, chain_id: ChainId) -> String {
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

pub async fn start_fetcher(
    client: Option<Client>,
    config: FetcherConfig,
    blocks_sink: mpsc::Sender<BlockWithTxHashes>,
    is_running: Arc<AtomicBool>,
) {
    let client = client.unwrap_or_else(|| Client::new());
    let max_num_threads = config.num_threads;
    let next_sink_block = Arc::new(AtomicU64::new(config.start_block_height));
    while is_running.load(Ordering::SeqCst) {
        let start_block_height = next_sink_block.load(Ordering::SeqCst);
        let next_fetch_block = Arc::new(AtomicU64::new(start_block_height));
        let last_block_height = fetch_last_block(&client, config.chain_id)
            .await
            .expect("Last block doesn't exist")
            .block
            .header
            .height;
        let is_backfill = last_block_height > start_block_height + max_num_threads;
        let num_threads = if is_backfill { max_num_threads } else { 1 };
        tracing::log::info!(
            target: LOG_TARGET,
            "Start fetching from block {} to block {} with {} threads. Backfill: {:?}",
            start_block_height,
            last_block_height,
            num_threads,
            is_backfill
        );
        // starting backfill with multiple threads
        let handles = (0..num_threads)
            .map(|thread_index| {
                let client = client.clone();
                let blocks_sink = blocks_sink.clone();
                let next_fetch_block = next_fetch_block.clone();
                let next_sink_block = next_sink_block.clone();
                let is_running = is_running.clone();
                let chain_id = config.chain_id;
                tokio::spawn(async move {
                    while is_running.load(Ordering::SeqCst) {
                        let block_height = next_fetch_block.fetch_add(1, Ordering::SeqCst);
                        if is_backfill && block_height > last_block_height {
                            break;
                        }
                        tracing::log::debug!(target: LOG_TARGET, "#{}: Fetching block: {}", thread_index, block_height);
                        let block =
                            fetch_block_by_height(&client, block_height, DEFAULT_TIMEOUT, chain_id).await;
                        while is_running.load(Ordering::SeqCst) {
                            let expected_block_height = next_sink_block.load(Ordering::SeqCst);
                            if expected_block_height < block_height {
                                tokio::time::sleep(Duration::from_millis(
                                    block_height - expected_block_height,
                                ))
                                .await;
                            } else {
                                tracing::log::debug!(target: LOG_TARGET, "#{}: Sending block: {}", thread_index, block_height);
                                break;
                            }
                        }
                        if !is_running.load(Ordering::SeqCst) {
                            break;
                        }
                        if let Some(block) = block {
                            blocks_sink.send(block).await.expect("Failed to send block");
                        } else {
                            tracing::log::debug!(target: LOG_TARGET, "#{}: Skipped block: {}", thread_index, block_height);
                        }
                        next_sink_block.fetch_add(1, Ordering::SeqCst);
                    }
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle.await.expect("Failed to join fetching thread");
        }
    }
}
