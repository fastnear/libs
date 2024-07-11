use fastnear_flat_state::filter::FlatStateFilter;
use fastnear_flat_state::state::FlatState;

mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let state_dump_path = utils::input("Enter state dump path", Some("./res/state_dump"))?;
    let save_state_path = utils::input("Enter path to save state (optional)", None)?;

    println!("Loading state...");
    let state = FlatState::from_state_dump(FlatStateFilter::full(), &state_dump_path)
        .await
        .map_err(|e| format!("{:?}", e))?;

    utils::print_state_info(&state);

    if !save_state_path.is_empty() {
        println!("Saving state...");
        state
            .save(&save_state_path)
            .map_err(|e| format!("{:?}", e))?;
        println!("State saved to: {}", save_state_path);
    }

    Ok(())
}
