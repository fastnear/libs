use crate::data::FlatStateData;
use crate::filter::FlatStateFilter;
use crate::state::*;
use fastnear_neardata_fetcher::fetcher::{fetch_block_by_height, new_reqwest_client};
use fastnear_primitives::near_primitives::block_header::BlockHeader;
use fastnear_primitives::near_primitives::state_record::{state_record_to_account_id, StateRecord};
use fastnear_primitives::near_primitives::views::{BlockHeaderInnerLiteView, StateChangeValueView};
use fastnear_primitives::types::ChainId;
use near_chain_configs::{
    stream_records_from_file, Genesis, GenesisValidationMode, GENESIS_CONFIG_FILENAME,
};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

const RECORDS_FILE: &str = "records.json";

const DEFAULT_FETCHER_TIMEOUT: Duration = Duration::from_secs(10);

impl FlatState {
    pub async fn from_state_dump(filter: FlatStateFilter, path: &str) -> FlatStateResult<Self> {
        let path = Path::new(path);
        let genesis = Genesis::from_file(
            path.join(GENESIS_CONFIG_FILENAME),
            GenesisValidationMode::UnsafeFast,
        )
        .map_err(|e| {
            FlatStateError::StateDumpError(format!(
                "Failed to load genesis config from file: {}",
                e
            ))
        })?;

        let chain_id = ChainId::try_from(genesis.config.chain_id.clone()).map_err(|e| {
            FlatStateError::StateDumpError(format!(
                "Failed to parse chain id from genesis config: {}",
                e
            ))
        })?;

        let block = fetch_block_by_height(
            &new_reqwest_client(),
            genesis.config.genesis_height,
            DEFAULT_FETCHER_TIMEOUT,
            chain_id,
        )
        .await
        .expect("State dump block is missing");

        let block_hash = block.block.header.hash;
        let block_header: BlockHeaderInnerLiteView = BlockHeader::from(block.block.header).into();

        let mut data = FlatStateData::default();

        let reader = BufReader::new(
            File::open(path.join(RECORDS_FILE)).expect("error while opening records file"),
        );
        tracing::info!(target: LOG_TARGET, "Reading records file");

        stream_records_from_file(reader, |record| {
            if !filter.is_account_allowed(state_record_to_account_id(&record)) {
                return;
            }
            let state_change_value = match record {
                StateRecord::Account {
                    account_id,
                    account,
                } => StateChangeValueView::AccountUpdate {
                    account_id,
                    account: account.into(),
                },
                StateRecord::Data {
                    account_id,
                    data_key,
                    value,
                } => StateChangeValueView::DataUpdate {
                    account_id,
                    key: data_key,
                    value,
                },
                StateRecord::Contract { account_id, code } => {
                    StateChangeValueView::ContractCodeUpdate { account_id, code }
                }
                StateRecord::AccessKey {
                    account_id,
                    public_key,
                    access_key,
                } => StateChangeValueView::AccessKeyUpdate {
                    account_id,
                    public_key,
                    access_key: access_key.into(),
                },
                StateRecord::PostponedReceipt(_)
                | StateRecord::ReceivedData { .. }
                | StateRecord::DelayedReceipt(..) => {
                    return;
                }
            };
            data.apply_state_change(state_change_value);
        })
        .expect("error while streaming records");

        Ok(Self {
            config: FlatStateConfig { chain_id, filter },
            block_header,
            block_hash,
            data,
        })
    }
}
