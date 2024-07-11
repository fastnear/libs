use fastnear_flat_state::state::FlatState;
mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let state = FlatState::load("./res/v1/state.borsh").unwrap();

    let path = tempfile::NamedTempFile::new()?.into_temp_path();
    state.save(path.to_str().unwrap()).unwrap();

    let state2 = FlatState::load(path.to_str().unwrap()).unwrap();

    assert_eq!(
        format!("{:?}", state),
        format!("{:?}", state2),
        "Saved and loaded states don't match"
    );

    utils::print_state_info(&state2);

    Ok(())
}
