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

    utils::print_state_info(&state);

    Ok(())
}
