use fastnear_flat_state::filter::FlatStateFilter;
use fastnear_flat_state::state::FlatState;

mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let state_dump_path = utils::input("Enter state dump path", Some("./res/state_dump"))?;

    let state = FlatState::from_state_dump(FlatStateFilter::full(), &state_dump_path)
        .await
        .map_err(|e| format!("{:?}", e))?;

    println!("Block Hash: {}", state.block_hash);
    println!("Block Height: {}", state.block_header.height);

    println!("Num Accounts: {}", state.data.accounts.len());

    if state.data.accounts.len() > 0 {
        let account_id = state.data.accounts.keys().next().unwrap();
        println!("First Account: {}", account_id);
        utils::print_account_info(&state, account_id);
    }

    Ok(())
}
