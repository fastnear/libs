use fastnear_primitives::near_primitives::borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_primitives::types::AccountId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct FlatStateFilter {
    pub accounts: HashSet<AccountId>,
    pub account_ranges: Vec<(Option<AccountId>, Option<AccountId>)>,
}

impl FlatStateFilter {
    pub fn empty() -> Self {
        Self {
            accounts: HashSet::new(),
            account_ranges: Vec::new(),
        }
    }

    pub fn full() -> Self {
        Self {
            accounts: HashSet::new(),
            account_ranges: vec![(None, None)],
        }
    }

    pub fn from_accounts(accounts: &[AccountId]) -> Self {
        Self {
            accounts: accounts.iter().cloned().collect(),
            account_ranges: Vec::new(),
        }
    }

    pub fn is_account_allowed(&self, account: &AccountId) -> bool {
        if self.accounts.contains(account) {
            return true;
        }
        for (start, end) in &self.account_ranges {
            if (start.is_none() || start.as_ref().unwrap() <= account)
                && (end.is_none() || end.as_ref().unwrap() >= account)
            {
                return true;
            }
        }
        false
    }
}
