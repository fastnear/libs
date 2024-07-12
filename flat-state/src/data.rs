use borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_indexer_primitives::types::AccountId;
use fastnear_primitives::near_primitives::account::{AccessKey, Account};
use fastnear_primitives::near_primitives::views::StateChangeValueView;
use near_crypto::PublicKey;
use std::collections::HashMap;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlatAccount {
    pub account: Account,
    pub access_keys: Vec<(PublicKey, AccessKey)>,
    pub data: Option<HashMap<Vec<u8>, Vec<u8>>>,
    pub contract_code: Option<Vec<u8>>,
}

impl FlatAccount {
    pub fn new(account: Account) -> Self {
        Self {
            account,
            access_keys: Vec::new(),
            data: None,
            contract_code: None,
        }
    }
}

#[derive(Default, Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlatStateData {
    pub accounts: HashMap<AccountId, FlatAccount>,
}

fn vec_insert<K, V>(vec: &mut Vec<(K, V)>, key: K, value: V)
where
    K: Eq,
{
    if let Some((_, v)) = vec.iter_mut().find(|(k, _)| *k == key) {
        *v = value;
    } else {
        vec.push((key, value));
    }
}

fn vec_remove<K, V>(vec: &mut Vec<(K, V)>, key: &K)
where
    K: Eq,
{
    vec.retain(|(k, _)| k != key);
}

impl FlatStateData {
    pub fn apply_state_change(&mut self, state_change_value: StateChangeValueView) {
        match state_change_value {
            StateChangeValueView::AccountUpdate {
                account_id,
                account,
            } => {
                if let Some(flat_account) = self.accounts.get_mut(&account_id) {
                    flat_account.account = account.into();
                } else {
                    self.accounts
                        .insert(account_id, FlatAccount::new(account.into()));
                }
            }
            StateChangeValueView::AccountDeletion { account_id } => {
                self.accounts.remove(&account_id);
            }
            StateChangeValueView::DataUpdate {
                account_id,
                key,
                value,
            } => {
                self.accounts.entry(account_id).and_modify(|entry| {
                    entry
                        .data
                        .get_or_insert_with(HashMap::new)
                        .insert(key.into(), value.into());
                });
            }

            StateChangeValueView::AccessKeyUpdate {
                account_id,
                public_key,
                access_key,
            } => {
                self.accounts.entry(account_id).and_modify(|entry| {
                    vec_insert(&mut entry.access_keys, public_key, access_key.into());
                });
            }
            StateChangeValueView::AccessKeyDeletion {
                account_id,
                public_key,
            } => {
                self.accounts.entry(account_id).and_modify(|entry| {
                    vec_remove(&mut entry.access_keys, &public_key);
                });
            }
            StateChangeValueView::DataDeletion { account_id, key } => {
                self.accounts.entry(account_id).and_modify(|entry| {
                    let key: Vec<u8> = key.into();
                    let is_empty = entry
                        .data
                        .as_mut()
                        .map(|data| {
                            data.remove(&key);
                            data.is_empty()
                        })
                        .unwrap_or(true);
                    if is_empty {
                        entry.data = None;
                    }
                });
            }
            StateChangeValueView::ContractCodeUpdate { account_id, code } => {
                self.accounts.entry(account_id).and_modify(|entry| {
                    entry.contract_code = Some(code.into());
                });
            }
            StateChangeValueView::ContractCodeDeletion { account_id } => {
                self.accounts.entry(account_id).and_modify(|entry| {
                    entry.contract_code = None;
                });
            }
        }
    }
}
