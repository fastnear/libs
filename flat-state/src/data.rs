use borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_indexer_primitives::types::AccountId;
use fastnear_primitives::near_primitives::account::{AccessKey, AccessKeyPermission, Account};
use fastnear_primitives::near_primitives::types::Nonce;
use fastnear_primitives::near_primitives::views::StateChangeValueView;
use near_crypto::{ED25519PublicKey, PublicKey};
use std::collections::HashMap;
use std::io::Read;

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct BorshifiedED25519PublicKey(pub ED25519PublicKey);

impl BorshSerialize for BorshifiedED25519PublicKey {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.0 .0)
    }
}

impl BorshDeserialize for BorshifiedED25519PublicKey {
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self(ED25519PublicKey(
            BorshDeserialize::deserialize_reader(reader)?,
        )))
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct AccountWithKey {
    pub account: Account,
    pub ed_full_key: Option<(BorshifiedED25519PublicKey, Nonce)>,
}

#[derive(Default, Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlatStateData {
    pub access_keys: HashMap<AccountId, HashMap<PublicKey, AccessKey>>,
    pub accounts: HashMap<AccountId, AccountWithKey>,
    pub data: HashMap<AccountId, HashMap<Vec<u8>, Vec<u8>>>,
    pub contracts_code: HashMap<AccountId, Vec<u8>>,
}

impl FlatStateData {
    pub fn apply_state_change(&mut self, state_change_value: StateChangeValueView) {
        match state_change_value {
            StateChangeValueView::AccountUpdate {
                account_id,
                account,
            } => {
                let account: Account = account.into();
                if let Some(ak) = self.accounts.get_mut(&account_id) {
                    ak.account = account;
                } else {
                    self.accounts.insert(
                        account_id,
                        AccountWithKey {
                            account,
                            ed_full_key: None,
                        },
                    );
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
                let access_key: AccessKey = access_key.into();
                let is_full_access =
                    matches!(access_key.permission, AccessKeyPermission::FullAccess);
                match &public_key {
                    PublicKey::ED25519(pk) => {
                        if let Some(ak) = self.accounts.get_mut(&account_id) {
                            let ed_full_key = ak.ed_full_key.take();
                            if let Some((key, _nonce)) = ed_full_key {
                                if &key.0 == pk {
                                    if is_full_access {
                                        ak.ed_full_key = Some((key, access_key.nonce));
                                        return;
                                    } else {
                                        ak.ed_full_key = None;
                                    }
                                }
                            } else {
                                if is_full_access {
                                    ak.ed_full_key = Some((
                                        BorshifiedED25519PublicKey(pk.clone()),
                                        access_key.nonce,
                                    ));
                                    return;
                                }
                            }
                        }
                    }
                    _ => {}
                };
                self.access_keys
                    .entry(account_id)
                    .or_insert_with(HashMap::new)
                    .insert(public_key, access_key);
            }
            StateChangeValueView::AccessKeyDeletion {
                account_id,
                public_key,
            } => {
                match &public_key {
                    PublicKey::ED25519(pk) => {
                        if let Some(ak) = self.accounts.get_mut(&account_id) {
                            let ed_full_key = ak.ed_full_key.take();
                            if let Some((key, _nonce)) = ed_full_key {
                                if &key.0 == pk {
                                    ak.ed_full_key = None;
                                    return;
                                }
                            }
                        }
                    }
                    _ => {}
                };
                let is_empty = {
                    let entry = self
                        .access_keys
                        .entry(account_id.clone())
                        .or_insert_with(HashMap::new);
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
