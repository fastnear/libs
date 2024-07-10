use fastnear_primitives::near_primitives::borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_primitives::types::AccountId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct FlatStateFilter {
    accounts: HashSet<AccountId>,
    account_ranges: Vec<(Option<AccountId>, Option<AccountId>)>,
}

impl FlatStateFilter {
    pub fn new() -> Self {
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

    pub fn add_account(&mut self, account: AccountId) {
        self.accounts.insert(account);
    }

    pub fn add_account_range(&mut self, start: Option<AccountId>, end: Option<AccountId>) {
        self.account_ranges.push((start, end));
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
