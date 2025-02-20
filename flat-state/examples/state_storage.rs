use fastnear_flat_state::state::FlatState;
mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let state = FlatState::load("./res/v1/state.borsh").unwrap();

    let path = tempfile::NamedTempFile::new()?.into_temp_path();
    state.save(path.to_str().unwrap()).unwrap();

    let state_loaded = FlatState::load(path.to_str().unwrap()).unwrap();
    //
    // assert_eq!(state.config, state_loaded.config, "Configs don't match");
    //     format!("{:?}", state),
    //     format!("{:?}", state_loaded),
    //     "Saved and loaded states don't match"
    // );
    //
    utils::print_state_info(&state_loaded);

    Ok(())
}
