use borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_indexer_primitives::types::AccountId;
use fastnear_primitives::near_primitives::account::{AccessKey, Account};
use fastnear_primitives::near_primitives::views::StateChangeValueView;
use near_crypto::PublicKey;
use std::collections::HashMap;

#[derive(Default, Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlatStateData {
    pub access_keys: HashMap<AccountId, Vec<(PublicKey, AccessKey)>>,
    pub accounts: HashMap<AccountId, Account>,
    pub data: HashMap<AccountId, HashMap<Vec<u8>, Vec<u8>>>,
    pub contracts_code: HashMap<AccountId, Vec<u8>>,
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
                    .or_insert_with(HashMap::new)
                    .insert(key.into(), value.into());
            }

            StateChangeValueView::AccessKeyUpdate {
                account_id,
                public_key,
                access_key,
            } => {
                vec_insert(
                    self.access_keys.entry(account_id).or_insert_with(Vec::new),
                    public_key,
                    access_key.into(),
                );
            }
            StateChangeValueView::AccessKeyDeletion {
                account_id,
                public_key,
            } => {
                let is_empty = {
                    let entry = self
                        .access_keys
                        .entry(account_id.clone())
                        .or_insert_with(Vec::new);
                    vec_remove(entry, &public_key);
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
                        .or_insert_with(HashMap::new);
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
