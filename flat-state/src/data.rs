use borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_indexer_primitives::types::AccountId;
use fastnear_primitives::near_primitives::account::{AccessKey, Account};
use fastnear_primitives::near_primitives::views::StateChangeValueView;
use near_crypto::PublicKey;
use std::collections::{BTreeMap, HashMap};

#[derive(Default, Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlatStateData {
    access_keys: HashMap<AccountId, BTreeMap<PublicKey, AccessKey>>,
    accounts: HashMap<AccountId, Account>,
    data: HashMap<AccountId, BTreeMap<Vec<u8>, Vec<u8>>>,
    contracts_code: HashMap<AccountId, Vec<u8>>,
}

impl FlatStateData {
    pub fn apply_state_update(&mut self, state_change_value: StateChangeValueView) {
        match state_change_value {
            StateChangeValueView::AccountUpdate {
                account_id,
                account,
            } => {
                self.accounts.insert(account_id, account.into());
            }
            StateChangeValueView::AccountDeletion { account_id } => {
                self.accounts.remove(&account_id);
            }
            StateChangeValueView::DataUpdate {
                account_id,
                key,
                value,
            } => {
                self.data
                    .entry(account_id)
                    .or_insert_with(BTreeMap::new)
                    .insert(key.into(), value.into());
            }

            StateChangeValueView::AccessKeyUpdate {
                account_id,
                public_key,
                access_key,
            } => {
                self.access_keys
                    .entry(account_id)
                    .or_insert_with(BTreeMap::new)
                    .insert(public_key, access_key.into());
            }
            StateChangeValueView::AccessKeyDeletion {
                account_id,
                public_key,
            } => {
                let is_empty = {
                    let entry = self
                        .access_keys
                        .entry(account_id.clone())
                        .or_insert_with(BTreeMap::new);
                    entry.remove(&public_key);
                    entry.is_empty()
                };
                if is_empty {
                    self.access_keys.remove(&account_id);
                }
            }
            StateChangeValueView::DataDeletion { account_id, key } => {
                let is_empty = {
                    let entry = self
                        .data
                        .entry(account_id.clone())
                        .or_insert_with(BTreeMap::new);
                    let key: Vec<u8> = key.into();
                    entry.remove(&key);
                    entry.is_empty()
                };
                if is_empty {
                    self.data.remove(&account_id);
                }
            }
            StateChangeValueView::ContractCodeUpdate { account_id, code } => {
                self.contracts_code.insert(account_id, code);
            }
            StateChangeValueView::ContractCodeDeletion { account_id } => {
                self.contracts_code.remove(&account_id);
            }
        }
    }
}
