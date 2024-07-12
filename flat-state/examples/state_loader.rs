use fastnear_flat_state::state::FlatState;
mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let path = utils::input(
        "Enter path to state.borsh file: ",
        Some("./res/v1/state.borsh"),
    )?;
    let state = FlatState::load(&path).unwrap();

    utils::print_state_info(&state);

    Ok(())
}
