use crate::data::FlatStateData;
use crate::state::{FlatState, FlatStateConfig, FlatStateError, FlatStateResult};
use fastnear_primitives::near_indexer_primitives::types::{BlockId, BlockReference};
use fastnear_primitives::near_indexer_primitives::views::{
    BlockHeaderInnerLiteView, StateChangeValueView, StateItem,
};
use fastnear_primitives::near_primitives;
use fastnear_primitives::near_primitives::block::BlockHeader;
use near_jsonrpc_primitives::types::query::{QueryResponseKind, RpcQueryError};

impl FlatState {
    pub async fn fetch_from_rpc(
        config: FlatStateConfig,
        rpc_url: String,
        block_reference: BlockReference,
    ) -> FlatStateResult<Self> {
        use near_jsonrpc_client::{methods, JsonRpcClient};

        if !config.filter.account_ranges.is_empty() {
            return Err(FlatStateError::FilterError(
                "Account ranges are not supported with RPC initialization".to_string(),
            ));
        }
        if config.filter.accounts.is_empty() {
            return Err(FlatStateError::FilterError(
                "The filter should contain at least one account ID with RPC initialization"
                    .to_string(),
            ));
        }

        let client = JsonRpcClient::connect(rpc_url);
        let block_request = methods::block::RpcBlockRequest { block_reference };
        let block = client
            .call(&block_request)
            .await
            .map_err(|e| FlatStateError::RpcError(format!("Failed to fetch block: {:?}", e)))?;

        let block_hash = block.header.hash;
        let block_header: BlockHeaderInnerLiteView = BlockHeader::from(block.header).into();

        let block_reference = BlockReference::BlockId(BlockId::Hash(block_hash));

        let mut state_change_values = Vec::new();
        for account_id in config.filter.accounts.iter() {
            let account = client
                .call(methods::query::RpcQueryRequest {
                    block_reference: block_reference.clone(),
                    request: near_primitives::views::QueryRequest::ViewAccount {
                        account_id: account_id.clone(),
                    },
                })
                .await
                .map(Some)
                .or_else(|e| {
                    if let Some(RpcQueryError::UnknownAccount { .. }) = e.handler_error() {
                        Ok(None)
                    } else {
                        Err(e)
                    }
                })
                .map_err(|e| {
                    FlatStateError::RpcError(format!(
                        "Failed to fetch account {}: {:?}",
                        account_id, e
                    ))
                })?;
            if let Some(account) = account {
                match account.kind {
                    QueryResponseKind::ViewAccount(account) => {
                        state_change_values.push(StateChangeValueView::AccountUpdate {
                            account_id: account_id.clone(),
                            account,
                        });
                    }
                    _ => {
                        return Err(FlatStateError::RpcError(format!(
                            "Unexpected response for account request {}: {:?}",
                            account_id, account
                        )));
                    }
                }
            } else {
                continue;
            }

            let access_keys = client
                .call(methods::query::RpcQueryRequest {
                    block_reference: block_reference.clone(),
                    request: near_primitives::views::QueryRequest::ViewAccessKeyList {
                        account_id: account_id.clone(),
                    },
                })
                .await
                .map_err(|e| {
                    FlatStateError::RpcError(format!(
                        "Failed to fetch access keys {}: {:?}",
                        account_id, e
                    ))
                })?;
            match access_keys.kind {
                QueryResponseKind::AccessKeyList(access_keys) => {
                    for access_key in access_keys.keys {
                        state_change_values.push(StateChangeValueView::AccessKeyUpdate {
                            account_id: account_id.clone(),
                            public_key: access_key.public_key,
                            access_key: access_key.access_key,
                        });
                    }
                }
                _ => {
                    return Err(FlatStateError::RpcError(format!(
                        "Unexpected response for access keys request {}: {:?}",
                        account_id, access_keys
                    )));
                }
            }

            let contract_code = client
                .call(methods::query::RpcQueryRequest {
                    block_reference: block_reference.clone(),
                    request: near_primitives::views::QueryRequest::ViewCode {
                        account_id: account_id.clone(),
                    },
                })
                .await
                .map(Some)
                .or_else(|e| {
                    if let Some(RpcQueryError::NoContractCode { .. }) = e.handler_error() {
                        Ok(None)
                    } else {
                        Err(e)
                    }
                })
                .map_err(|e| {
                    FlatStateError::RpcError(format!(
                        "Failed to fetch contract code {}: {:?}",
                        account_id, e
                    ))
                })?;
            if let Some(contract_code) = contract_code {
                match contract_code.kind {
                    QueryResponseKind::ViewCode(contract_code) => {
                        state_change_values.push(StateChangeValueView::ContractCodeUpdate {
                            account_id: account_id.clone(),
                            code: contract_code.code,
                        });
                    }
                    _ => {
                        return Err(FlatStateError::RpcError(format!(
                            "Unexpected response for contract code request {}: {:?}",
                            account_id, contract_code
                        )));
                    }
                };
            }

            let state = client
                .call(methods::query::RpcQueryRequest {
                    block_reference: block_reference.clone(),
                    request: near_primitives::views::QueryRequest::ViewState {
                        account_id: account_id.clone(),
                        prefix: vec![].into(),
                        include_proof: false,
                    },
                })
                .await
                .map(Some)
                .or_else(|e| {
                    // TODO: Temporary fix for the read-rpc inconsistency
                    if let Some(RpcQueryError::UnknownAccount { .. }) = e.handler_error() {
                        Ok(None)
                    } else {
                        Err(e)
                    }
                })
                .map_err(|e| {
                    FlatStateError::RpcError(format!(
                        "Failed to fetch state {}: {:?}",
                        account_id, e
                    ))
                })?;
            if let Some(state) = state {
                match state.kind {
                    QueryResponseKind::ViewState(state) => {
                        for StateItem { key, value } in state.values {
                            state_change_values.push(StateChangeValueView::DataUpdate {
                                account_id: account_id.clone(),
                                key,
                                value,
                            });
                        }
                    }
                    _ => {
                        return Err(FlatStateError::RpcError(format!(
                            "Unexpected response for state request {}: {:?}",
                            account_id, state
                        )));
                    }
                }
            }
        }
        let mut data = FlatStateData::default();
        for state_change_value in state_change_values {
            data.apply_state_update(state_change_value);
        }

        Ok(Self {
            config,
            block_header,
            block_hash,
            data,
        })
    }
}
