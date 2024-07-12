use fastnear_flat_state::filter::FlatStateFilter;
use fastnear_flat_state::state::{FlatState, FlatStateConfig};
use fastnear_primitives::near_indexer_primitives::types::Finality;
use fastnear_primitives::near_primitives::types::{AccountId, BlockReference};
use fastnear_primitives::types::ChainId;

mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let block_reference = BlockReference::Finality(Finality::Final);

    let chain_id = ChainId::try_from(utils::input("Enter chain ID", Some("mainnet"))?)
        .expect("Invalid chain ID");
    let rpc_url = utils::input("Enter RPC URL", Some(utils::DEFAULT_RPC_URL))?;
    let account_id = AccountId::try_from(utils::input("Enter account ID", Some("zavodil.near"))?)
        .expect("Invalid account ID");
    let save_state_path = utils::input("Enter path to save state (optional)", None)?;

    println!("Fetching state...");

    let state = FlatState::fetch_from_rpc(
        FlatStateConfig {
            chain_id,
            filter: FlatStateFilter::from_accounts(&[account_id.clone()]),
        },
        rpc_url,
        block_reference,
    )
    .await
    .map_err(|e| format!("{:?}", e))?;

    println!("Block Hash: {}", state.block_hash);
    println!("Block Height: {}", state.block_header.height);

    utils::print_account_info(&state, &account_id);

    if !save_state_path.is_empty() {
        println!("Saving state...");
        state
            .save(&save_state_path)
            .map_err(|e| format!("{:?}", e))?;
        println!("State saved to: {}", save_state_path);
    }

    Ok(())
}
