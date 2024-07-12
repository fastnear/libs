#![allow(unused)]

use fastnear_flat_state::state::FlatState;
use fastnear_primitives::near_primitives::types::AccountId;
use std::io::{self, Write};

pub const DEFAULT_RPC_URL: &str = "https://beta.rpc.mainnet.near.org";

pub fn input(query: &str, default: Option<&str>) -> io::Result<String> {
    print!(
        "{}{}: ",
        query,
        default.map_or("".to_string(), |d| format!(" ({})", d))
    );
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_owned();
    if input.is_empty() {
        return Ok(default.unwrap_or_default().to_string());
    }
    Ok(input)
}

pub fn print_account_info(state: &FlatState, account_id: &AccountId) {
    let account = state.data.accounts.get(account_id);
    if let Some(account) = account {
        println!("Account: {:#?}", account.account);
        println!("Num Access keys: {}", account.access_keys.len());
        println!(
            "Contract code length: {}",
            account.contract_code.as_ref().map_or(0, |d| d.len())
        );
        println!(
            "Num Data keys: {}",
            account.data.as_ref().map_or(0, |d| d.len())
        );
    } else {
        println!("Account not found: {}", account_id);
    }
}

pub fn print_state_info(state: &FlatState) {
    println!("Block Hash: {}", state.block_hash);
    println!("Block Height: {}", state.block_header.height);

    println!("Num Accounts: {}", state.data.accounts.len());

    if state.data.accounts.len() > 0 {
        let account_id = state.data.accounts.keys().next().unwrap();
        println!("First Account: {}", account_id);
        print_account_info(&state, account_id);
    }
}
