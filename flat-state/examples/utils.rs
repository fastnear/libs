#![allow(unused)]

use fastnear_flat_state::state::FlatState;
use fastnear_primitives::near_primitives::types::AccountId;
use std::io::{self, Write};

pub const DEFAULT_RPC_URL: &str = "https://archival-rpc.mainnet.pagoda.co";

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
    println!("Account: {:#?}", state.data.accounts.get(account_id));
    println!(
        "Num Access keys: {}",
        state
            .data
            .access_keys
            .get(account_id)
            .map_or(0, |d| d.len())
    );
    println!(
        "Contract code length: {}",
        state
            .data
            .contracts_code
            .get(account_id)
            .map_or(0, |d| d.len())
    );
    println!(
        "Num Data keys: {}",
        state.data.data.get(account_id).map_or(0, |d| d.len())
    );
}

pub fn print_state_info(state: &FlatState) {
    println!("Block Hash: {}", state.block_hash);
    println!("Block Height: {}", state.block_header.height);

    println!("Num Accounts: {}", state.data.accounts.len());
    println!(
        "Num Accounts with Access keys: {}",
        state.data.access_keys.len()
    );
    println!("Num Accounts with Data: {}", state.data.data.len());
    println!(
        "Num Accounts with Contract code: {}",
        state.data.contracts_code.len()
    );

    if state.data.accounts.len() > 0 {
        let account_id = state.data.accounts.keys().next().unwrap();
        println!("First Account: {}", account_id);
        print_account_info(&state, account_id);
    }
}
