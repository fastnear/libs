use crate::*;

use fastnear_primitives::near_primitives::types::Finality;
use std::time::Duration;

pub type BlockResult = Result<Option<BlockWithTxHashes>, FetchError>;

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
    pub start_block_height: Option<BlockHeight>,
    pub chain_id: ChainId,
    pub timeout_duration: Option<Duration>,
    pub retry_duration: Option<Duration>,
    pub disable_archive_sync: bool,
    /// The Bearer token to use for authentication
    pub auth_bearer_token: Option<String>,
    pub finality: Finality,
}
