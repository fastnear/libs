use borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_indexer_primitives::types::AccountId;
use fastnear_primitives::near_primitives::account::{AccessKey, Account};
use fastnear_primitives::near_primitives::views::StateChangeValueView;
use near_crypto::{ED25519PublicKey, KeyType, PublicKey, Secp256K1PublicKey};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read, Write};
use std::ops::Deref;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BoxedPublicKey {
    /// 256 bit elliptic curve based public-key.
    ED25519(Box<ED25519PublicKey>),
    /// 512 bit elliptic curve based public-key used in Bitcoin's public-key cryptography.
    SECP256K1(Box<Secp256K1PublicKey>),
}

impl BorshSerialize for BoxedPublicKey {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            BoxedPublicKey::ED25519(public_key) => {
                BorshSerialize::serialize(&0u8, writer)?;
                writer.write_all(&public_key.0)?;
            }
            BoxedPublicKey::SECP256K1(public_key) => {
                BorshSerialize::serialize(&1u8, writer)?;
                writer.write_all(public_key.deref().as_ref())?;
            }
        }
        Ok(())
    }
}

impl BorshDeserialize for BoxedPublicKey {
    fn deserialize_reader<R: Read>(rd: &mut R) -> std::io::Result<Self> {
        let public_key = PublicKey::try_from_reader(rd)?;
        Ok(public_key.into())
    }
}

impl From<PublicKey> for BoxedPublicKey {
    fn from(public_key: PublicKey) -> Self {
        match public_key {
            PublicKey::ED25519(ed25519) => BoxedPublicKey::ED25519(ed25519.into()),
            PublicKey::SECP256K1(secp256k1) => BoxedPublicKey::SECP256K1(secp256k1.into()),
        }
    }
}

impl From<BoxedPublicKey> for PublicKey {
    fn from(boxed_public_key: BoxedPublicKey) -> Self {
        match boxed_public_key {
            BoxedPublicKey::ED25519(ed25519) => PublicKey::ED25519(*ed25519),
            BoxedPublicKey::SECP256K1(secp256k1) => PublicKey::SECP256K1(*secp256k1),
        }
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct FlatAccount {
    pub account: Account,
    pub access_keys: Vec<(BoxedPublicKey, AccessKey)>,
    pub data: Option<Box<HashMap<Vec<u8>, Vec<u8>>>>,
    pub contract_code: Option<Box<Vec<u8>>>,
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
                        .get_or_insert_with(|| Box::new(HashMap::new()))
                        .insert(key.into(), value.into());
                });
            }

            StateChangeValueView::AccessKeyUpdate {
                account_id,
                public_key,
                access_key,
            } => {
                self.accounts.entry(account_id).and_modify(|entry| {
                    vec_insert(&mut entry.access_keys, public_key.into(), access_key.into());
                });
            }
            StateChangeValueView::AccessKeyDeletion {
                account_id,
                public_key,
            } => {
                self.accounts.entry(account_id).and_modify(|entry| {
                    vec_remove(&mut entry.access_keys, &public_key.into());
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
