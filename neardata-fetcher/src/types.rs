use crate::*;

use fastnear_primitives::near_primitives::types::Finality;
use std::time::Duration;

pub type BlockResult = Result<Option<BlockWithTxHashes>, FetchError>;

#[derive(Debug)]
pub enum FetchError {
    ReqwestError(reqwest::Error),
    RedirectError,
}

impl From<reqwest::Error> for FetchError {
    fn from(error: reqwest::Error) -> Self {
        FetchError::ReqwestError(error)
    }
}

#[derive(Debug, Clone)]
pub struct FetcherConfig {
    pub num_threads: u64,
    /// The start block height to fetch from (inclusive)
    pub start_block_height: Option<BlockHeight>,
    /// The end block height to fetch up to (inclusive)
    pub end_block_height: Option<BlockHeight>,
    pub chain_id: ChainId,
    pub timeout_duration: Option<Duration>,
    pub retry_duration: Option<Duration>,
    pub disable_archive_sync: bool,
    /// The Bearer token to use for authentication
    pub auth_bearer_token: Option<String>,
    pub finality: Finality,
    pub enable_r2_archive_sync: bool,
}

#[derive(Debug, Clone)]
pub struct FetcherConfigBuilder {
    config: FetcherConfig,
}

impl FetcherConfigBuilder {
    pub fn new() -> Self {
        FetcherConfigBuilder {
            config: FetcherConfig {
                num_threads: 4,
                start_block_height: None,
                end_block_height: None,
                chain_id: ChainId::Mainnet,
                timeout_duration: None,
                retry_duration: None,
                disable_archive_sync: false,
                auth_bearer_token: None,
                finality: Finality::Final,
                enable_r2_archive_sync: false,
            },
        }
    }

    pub fn num_threads(mut self, num_threads: u64) -> Self {
        self.config.num_threads = num_threads;
        self
    }

    /// The start block height to fetch from (inclusive)
    pub fn start_block_height(mut self, start_block_height: BlockHeight) -> Self {
        self.config.start_block_height = Some(start_block_height);
        self
    }

    /// The end block height to fetch up to (inclusive)
    pub fn end_block_height(mut self, end_block_height: BlockHeight) -> Self {
        self.config.end_block_height = Some(end_block_height);
        self
    }

    pub fn chain_id(mut self, chain_id: ChainId) -> Self {
        self.config.chain_id = chain_id;
        self
    }

    pub fn timeout_duration(mut self, timeout_duration: Duration) -> Self {
        self.config.timeout_duration = Some(timeout_duration);
        self
    }

    pub fn retry_duration(mut self, retry_duration: Duration) -> Self {
        self.config.retry_duration = Some(retry_duration);
        self
    }

    pub fn disable_archive_sync(mut self, disable_archive_sync: bool) -> Self {
        self.config.disable_archive_sync = disable_archive_sync;
        self
    }

    pub fn auth_bearer_token(mut self, auth_bearer_token: String) -> Self {
        self.config.auth_bearer_token = Some(auth_bearer_token);
        self
    }

    pub fn finality(mut self, finality: Finality) -> Self {
        self.config.finality = finality;
        self
    }

    /// R2 endpoint has lower rate limits and should be used with authentication
    pub fn enable_r2_archive_sync(mut self, enable_r2_archive_sync: bool) -> Self {
        self.config.enable_r2_archive_sync = enable_r2_archive_sync;
        self
    }

    pub fn build(self) -> FetcherConfig {
        self.config
    }
}
