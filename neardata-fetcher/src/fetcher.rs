pub use crate::types::*;
pub use crate::utils::*;
use crate::*;
use fastnear_primitives::near_primitives::types::Finality;
use fastnear_primitives::near_primitives::views::BlockView;
use reqwest::ClientBuilder;
use std::io::Read;

#[derive(Debug)]
struct InterruptedError;

type InterruptibleResult<T> = Result<T, InterruptedError>;

pub const MAX_REDIRECTS: usize = 5;

#[derive(Clone)]
struct Fetcher {
    client: Client,
    config: FetcherConfig,
    is_running: Arc<AtomicBool>,
}

impl Fetcher {
    pub async fn fetch<T>(&self, url: &str) -> Result<Option<T>, FetchError>
    where
        T: serde::de::DeserializeOwned,
    {
        // Manually handle redirects and adding auth headers
        let mut url = url.to_string();
        for _ in 0..MAX_REDIRECTS {
            let mut request = self.client.get(&url);
            if let Some(token) = &self.config.auth_bearer_token {
                request = request.bearer_auth(token);
            }
            let response = request
                .timeout(self.config.timeout_duration.unwrap_or(DEFAULT_TIMEOUT))
                .send()
                .await?;

            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .ok_or(FetchError::RedirectError)?
                    .to_str()
                    .map_err(|_| FetchError::RedirectError)?;

                let parsed_current =
                    url::Url::parse(&url).map_err(|_| FetchError::RedirectError)?;

                // Resolve the location relative to the current URL
                url = parsed_current
                    .join(location)
                    .map_err(|_| FetchError::RedirectError)?
                    .to_string();
                continue;
            }

            return Ok(response.json().await?);
        }
        Err(FetchError::RedirectError)
    }

    pub async fn fetch_until_success<T>(&self, url: &str) -> InterruptibleResult<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        while self.is_running.load(Ordering::SeqCst) {
            match self.fetch(url).await {
                Ok(block) => return Ok(block),
                Err(FetchError::ReqwestError(err)) => {
                    tracing::log::warn!(target: LOG_TARGET, "Failed to fetch: {}", err);
                    tokio::time::sleep(
                        self.config.retry_duration.unwrap_or(DEFAULT_RETRY_DURATION),
                    )
                    .await;
                }
                Err(FetchError::RedirectError) => {
                    tracing::log::warn!(target: LOG_TARGET, "Redirect error");
                    tokio::time::sleep(
                        self.config.retry_duration.unwrap_or(DEFAULT_RETRY_DURATION),
                    )
                    .await;
                }
            }
        }
        Err(InterruptedError)
    }

    pub async fn fetch_block_until_success(
        &self,
        url: &str,
    ) -> InterruptibleResult<Option<BlockWithTxHashes>> {
        self.fetch_until_success(url).await
    }

    pub async fn fetch_last_block_headers(
        &self,
        finality: &Finality,
    ) -> InterruptibleResult<Option<BlockView>> {
        self.fetch_until_success(&target_url(
            &format!(
                "/v0/last_block/{}/headers",
                if finality == &Finality::Final {
                    "final"
                } else {
                    "optimistic"
                }
            ),
            self.config.chain_id,
        ))
        .await
    }

    pub async fn fetch_block_by_height(
        &self,
        height: BlockHeight,
        finality: &Finality,
    ) -> InterruptibleResult<Option<BlockWithTxHashes>> {
        self.fetch_block_until_success(&target_url(
            &format!(
                "/v0/block{}/{}",
                if finality == &Finality::Final {
                    ""
                } else {
                    "_opt"
                },
                height
            ),
            self.config.chain_id,
        ))
        .await
    }

    async fn fetch_archive(&self, url: &str) -> Result<Option<Vec<u8>>, FetchError> {
        let mut request = self.client.get(url);
        if let Some(token) = &self.config.auth_bearer_token {
            request = request.bearer_auth(token);
        }
        let response = request
            .timeout(self.config.timeout_duration.unwrap_or(DEFAULT_TIMEOUT))
            .send()
            .await?;
        if response.status() == 404 {
            return Ok(None);
        }
        Ok(response.bytes().await.map(|b| Some(b.to_vec()))?)
    }

    fn parse_archive(&self, archive: Vec<u8>) -> Result<Vec<BlockWithTxHashes>, String> {
        let archive = std::io::Cursor::new(archive);
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(archive));
        let mut blocks = Vec::new();
        for entry in archive.entries().map_err(|err| err.to_string())? {
            let mut entry = entry.map_err(|err| err.to_string())?;
            let mut content = Vec::new();
            entry
                .read_to_end(&mut content)
                .map_err(|err| err.to_string())?;
            let block: BlockWithTxHashes =
                serde_json::from_slice(&content).map_err(|err| err.to_string())?;
            blocks.push(block);
        }
        blocks.sort_by(|a, b| a.block.header.height.cmp(&b.block.header.height));
        Ok(blocks)
    }

    async fn fetch_blocks_from_archive(
        &self,
        archive_block_height: BlockHeight,
    ) -> InterruptibleResult<Vec<BlockWithTxHashes>> {
        let padded_block_height = format!("{:0>12}", archive_block_height);
        let suffix = &format!(
            "{}/{}/{}.tgz",
            &padded_block_height[..6],
            &padded_block_height[6..9],
            padded_block_height
        );
        let prefix = match self.config.chain_id {
            ChainId::Mainnet
                if self.config.enable_r2_archive_sync
                    && archive_block_height <= MAINNET_R2_LAST_BLOCK_HEIGHT =>
            {
                "https://archive.data.fastnear.com/mainnet/".to_string()
            }
            ChainId::Testnet
                if self.config.enable_r2_archive_sync
                    && archive_block_height <= TESTNET_ARCHIVE_LAST_BLOCK_HEIGHT =>
            {
                "https://archive.data.fastnear.com/testnet/".to_string()
            }
            ChainId::Mainnet => format!(
                "https://a{}.mainnet.neardata.xyz/raw/",
                MAINNET_ARCHIVE_BOUNDARIES
                    .iter()
                    .position(|&b| archive_block_height < b)
                    .unwrap_or(MAINNET_ARCHIVE_BOUNDARIES.len())
            ),
            ChainId::Testnet => "https://testnet.neardata.xyz/raw/".to_string(),
        };
        let url = format!("{}{}", prefix, suffix);
        tracing::log::debug!(target: LOG_TARGET, "#{}: Fetching archive url: {}", archive_block_height, url);
        while self.is_running.load(Ordering::SeqCst) {
            match self.fetch_archive(&url).await {
                Ok(Some(archive)) => match self.parse_archive(archive) {
                    Ok(blocks) => return Ok(blocks),
                    Err(err) => {
                        tracing::log::warn!(target: LOG_TARGET, "Failed to parse archive {} : {}", url, err);
                        tokio::time::sleep(
                            self.config.retry_duration.unwrap_or(DEFAULT_RETRY_DURATION),
                        )
                        .await;
                    }
                },
                Ok(None) => return Ok(Vec::new()),
                Err(FetchError::ReqwestError(err)) => {
                    tracing::log::warn!(target: LOG_TARGET, "Failed to fetch the archive: {}", err);
                    tokio::time::sleep(
                        self.config.retry_duration.unwrap_or(DEFAULT_RETRY_DURATION),
                    )
                    .await;
                }
                Err(FetchError::RedirectError) => {
                    tracing::log::warn!(target: LOG_TARGET, "Redirect error");
                    tokio::time::sleep(
                        self.config.retry_duration.unwrap_or(DEFAULT_RETRY_DURATION),
                    )
                    .await;
                }
            }
        }
        Err(InterruptedError)
    }
}

async fn archive_sync(
    fetcher: &Fetcher,
    blocks_sink: mpsc::Sender<BlockWithTxHashes>,
    start_block_height: BlockHeight,
    end_block_height: BlockHeight,
    next_sink_block: Arc<AtomicU64>,
) {
    tracing::log::info!(
        target: LOG_TARGET,
        "Start archive sync from block {} to block {} with {} threads.",
        start_block_height,
        end_block_height,
        fetcher.config.num_threads
    );
    let next_fetch_archive_height = Arc::new(AtomicU64::new(
        start_block_height / NUMBER_OF_BLOCKS_PER_ARCHIVE * NUMBER_OF_BLOCKS_PER_ARCHIVE,
    ));
    // starting backfill with multiple threads
    let handles = (0..fetcher.config.num_threads)
            .map(|thread_index| {
                let fetcher = fetcher.clone();
                let blocks_sink = blocks_sink.clone();
                let next_fetch_archive_height = next_fetch_archive_height.clone();
                let next_sink_block = next_sink_block.clone();
                tokio::spawn(async move {
                    while fetcher.is_running.load(Ordering::SeqCst) {
                        let archive_block_height = next_fetch_archive_height.fetch_add(NUMBER_OF_BLOCKS_PER_ARCHIVE, Ordering::SeqCst);
                        if archive_block_height >= end_block_height {
                            break;
                        }
                        tracing::log::debug!(target: LOG_TARGET, "#{}: Fetching archive: {}", thread_index, archive_block_height);
                        let blocks =
                            fetcher.fetch_blocks_from_archive(archive_block_height).await;
                        let mut expected_block_height = 0;
                        while fetcher.is_running.load(Ordering::SeqCst) {
                            expected_block_height = next_sink_block.load(Ordering::SeqCst);
                            if expected_block_height < archive_block_height {
                                tokio::time::sleep(Duration::from_millis(
                                    (archive_block_height - expected_block_height + NUMBER_OF_BLOCKS_PER_ARCHIVE - 1) / NUMBER_OF_BLOCKS_PER_ARCHIVE * NUMBER_OF_BLOCKS_PER_ARCHIVE,
                                ))
                                    .await;
                            } else {
                                tracing::log::debug!(target: LOG_TARGET, "#{}: Sending blocks from archive: {}", thread_index, archive_block_height);
                                break;
                            }
                        }
                        if !fetcher.is_running.load(Ordering::SeqCst) {
                            break;
                        }
                        for block in blocks.expect("Can't be interrupted error") {
                            // Skipping initial blocks from archive
                            if block.block.header.height < expected_block_height {
                                continue;
                            }
                            blocks_sink.send(block).await.expect("Failed to send block");
                        }
                        next_sink_block.swap(archive_block_height + NUMBER_OF_BLOCKS_PER_ARCHIVE, Ordering::SeqCst);
                    }
                })
            })
            .collect::<Vec<_>>();
    for handle in handles {
        handle.await.expect("Failed to join fetching thread");
    }
}

pub async fn start_fetcher(
    config: FetcherConfig,
    blocks_sink: mpsc::Sender<BlockWithTxHashes>,
    is_running: Arc<AtomicBool>,
) {
    let client = ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let fetcher = Fetcher {
        client,
        config,
        is_running,
    };
    let max_num_threads = fetcher.config.num_threads;
    let start_block_height = if let Some(start_block_height) = fetcher.config.start_block_height {
        start_block_height
    } else {
        let last_block_height = fetcher
            .fetch_last_block_headers(&fetcher.config.finality)
            .await;
        if let Err(InterruptedError) = last_block_height {
            return;
        }
        last_block_height
            .unwrap()
            .expect("Last block doesn't exist")
            .header
            .height
    };
    let end_block_height = fetcher.config.end_block_height.unwrap_or(std::u64::MAX);
    let next_sink_block = Arc::new(AtomicU64::new(start_block_height));
    while fetcher.is_running.load(Ordering::SeqCst) {
        let start_block_height = next_sink_block.load(Ordering::SeqCst);
        if start_block_height > end_block_height {
            break;
        }
        let last_block_height = fetcher
            .fetch_last_block_headers(&fetcher.config.finality)
            .await;
        if let Err(InterruptedError) = last_block_height {
            break;
        }
        let last_block_height = last_block_height
            .unwrap()
            .expect("Last block doesn't exist")
            .header
            .height;
        let last_block_height = std::cmp::min(last_block_height, end_block_height);
        let rounded_last_block_height =
            last_block_height / NUMBER_OF_BLOCKS_PER_ARCHIVE * NUMBER_OF_BLOCKS_PER_ARCHIVE;
        if !fetcher.config.disable_archive_sync
            && rounded_last_block_height > start_block_height + ARCHIVE_SYNC_THRESHOLD
        {
            archive_sync(
                &fetcher,
                blocks_sink.clone(),
                start_block_height,
                rounded_last_block_height,
                next_sink_block.clone(),
            )
            .await;
            continue;
        }
        let next_fetch_block = Arc::new(AtomicU64::new(start_block_height));
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
                let fetcher = fetcher.clone();
                let blocks_sink = blocks_sink.clone();
                let next_fetch_block = next_fetch_block.clone();
                let next_sink_block = next_sink_block.clone();
                tokio::spawn(async move {
                    while fetcher.is_running.load(Ordering::SeqCst) {
                        let block_height = next_fetch_block.fetch_add(1, Ordering::SeqCst);
                        if (is_backfill && block_height > last_block_height) || block_height > end_block_height {
                            break;
                        }
                        tracing::log::debug!(target: LOG_TARGET, "#{}: Fetching block: {}", thread_index, block_height);
                        let block =
                            fetcher.fetch_block_by_height(block_height, &fetcher.config.finality).await;
                        while fetcher.is_running.load(Ordering::SeqCst) {
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
                        if !fetcher.is_running.load(Ordering::SeqCst) {
                            break;
                        }
                        if let Some(block) = block.expect("Can't be interrupted error") {
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
