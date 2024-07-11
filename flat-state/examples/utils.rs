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
