use crate::filter::FlatStateFilter;
use fastnear_primitives::block_with_tx_hash::BlockWithTxHashes;
use fastnear_primitives::near_indexer_primitives::views::StateChangeValueView;
use fastnear_primitives::near_indexer_primitives::CryptoHash;
use fastnear_primitives::near_primitives::account::{AccessKey, Account};
use fastnear_primitives::near_primitives::block::BlockHeader;
use fastnear_primitives::near_primitives::borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_primitives::types::AccountId;
use fastnear_primitives::near_primitives::views::BlockHeaderInnerLiteView;
use fastnear_primitives::types::ChainId;
use near_crypto::PublicKey;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

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

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlatStateData {
    access_keys: HashMap<AccountId, BTreeMap<PublicKey, AccessKey>>,
    accounts: HashMap<AccountId, Account>,
    data: HashMap<AccountId, BTreeMap<Vec<u8>, Vec<u8>>>,
    contracts_code: HashMap<AccountId, Vec<u8>>,
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
        let data = &mut self.data;
        for shard in block.shards {
            for state_change in shard.state_changes {
                {
                    let account_id = match &state_change.value {
                        StateChangeValueView::AccountUpdate { account_id, .. } => account_id,
                        StateChangeValueView::AccountDeletion { account_id, .. } => account_id,
                        StateChangeValueView::AccessKeyUpdate { account_id, .. } => account_id,
                        StateChangeValueView::AccessKeyDeletion { account_id, .. } => account_id,
                        StateChangeValueView::DataUpdate { account_id, .. } => account_id,
                        StateChangeValueView::DataDeletion { account_id, .. } => account_id,
                        StateChangeValueView::ContractCodeUpdate { account_id, .. } => account_id,
                        StateChangeValueView::ContractCodeDeletion { account_id, .. } => account_id,
                    };

                    if !self.config.filter.is_account_allowed(account_id) {
                        continue;
                    }
                }
                match state_change.value {
                    StateChangeValueView::AccountUpdate {
                        account_id,
                        account,
                    } => {
                        data.accounts.insert(account_id, account.into());
                    }
                    StateChangeValueView::AccountDeletion { account_id } => {
                        data.accounts.remove(&account_id);
                    }
                    StateChangeValueView::DataUpdate {
                        account_id,
                        key,
                        value,
                    } => {
                        data.data
                            .entry(account_id)
                            .or_insert_with(BTreeMap::new)
                            .insert(key.into(), value.into());
                    }

                    StateChangeValueView::AccessKeyUpdate {
                        account_id,
                        public_key,
                        access_key,
                    } => {
                        data.access_keys
                            .entry(account_id)
                            .or_insert_with(BTreeMap::new)
                            .insert(public_key, access_key.into());
                    }
                    StateChangeValueView::AccessKeyDeletion {
                        account_id,
                        public_key,
                    } => {
                        let is_empty = {
                            let entry = data
                                .access_keys
                                .entry(account_id.clone())
                                .or_insert_with(BTreeMap::new);
                            entry.remove(&public_key);
                            entry.is_empty()
                        };
                        if is_empty {
                            data.access_keys.remove(&account_id);
                        }
                    }
                    StateChangeValueView::DataDeletion { account_id, key } => {
                        let is_empty = {
                            let entry = data
                                .data
                                .entry(account_id.clone())
                                .or_insert_with(BTreeMap::new);
                            let key: Vec<u8> = key.into();
                            entry.remove(&key);
                            entry.is_empty()
                        };
                        if is_empty {
                            data.data.remove(&account_id);
                        }
                    }
                    StateChangeValueView::ContractCodeUpdate { account_id, code } => {
                        data.contracts_code.insert(account_id, code);
                    }
                    StateChangeValueView::ContractCodeDeletion { account_id } => {
                        data.contracts_code.remove(&account_id);
                    }
                }
            }
        }
        Ok(())
    }
}
