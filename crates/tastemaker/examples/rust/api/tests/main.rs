//! The command: Cargo builds the server's executable for the tests of this target.

#[test]
fn the_server_is_built() {
    let built = std::path::Path::new(env!("CARGO_BIN_EXE_todo-api"));
    assert!(built.is_file(), "{} is built", built.display());
}
