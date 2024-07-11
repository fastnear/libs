use crate::data::FlatStateData;
use crate::filter::FlatStateFilter;
use fastnear_primitives::block_with_tx_hash::BlockWithTxHashes;
use fastnear_primitives::near_indexer_primitives::CryptoHash;
use fastnear_primitives::near_primitives::block::BlockHeader;
use fastnear_primitives::near_primitives::borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_primitives::views::BlockHeaderInnerLiteView;
use fastnear_primitives::types::ChainId;
use fastnear_primitives::utils::state_change_account_id;
use serde::{Deserialize, Serialize};

const LOG_TARGET: &str = "flat-state";

pub type FlatStateResult<T> = Result<T, FlatStateError>;

#[derive(Debug, Clone)]
pub enum FlatStateError {
    BlockHashMismatch,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct FlatStateConfig {
    pub chain_id: ChainId,
    pub storage_path: Option<String>,
    pub filter: FlatStateFilter,
}

pub struct FlatState {
    config: FlatStateConfig,

    block_header: BlockHeaderInnerLiteView,
    block_hash: CryptoHash,

    data: FlatStateData,
}

impl FlatState {
    pub fn apply_block(&mut self, block: BlockWithTxHashes) -> FlatStateResult<()> {
        let prev_hash = block.block.header.prev_hash;
        if self.block_hash != prev_hash {
            tracing::log::error!(
                target: LOG_TARGET,
                "Block hash mismatch: expected {}, got {}",
                self.block_hash,
                prev_hash
            );
            return Err(FlatStateError::BlockHashMismatch);
        }
        self.block_hash = block.block.header.hash;
        self.block_header = BlockHeader::from(block.block.header).into();
        for shard in block.shards {
            for state_change in shard.state_changes {
                if self
                    .config
                    .filter
                    .is_account_allowed(state_change_account_id(&state_change.value))
                {
                    self.data.apply_state_update(state_change.value);
                }
            }
        }
        Ok(())
    }
}
