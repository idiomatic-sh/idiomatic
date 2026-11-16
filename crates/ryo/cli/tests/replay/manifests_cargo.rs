//! Ryo says what Cargo says of the manifests of a workspace (`cargo::cargo_differences`): the members and the dependencies of a
//! workspace written here.

use crate::cargo::cargo_differences;

#[test]
fn ryo_says_what_cargo_says_of_the_manifests_of_a_workspace() {
    let _beside = crate::beside();
    let root = crate::manifests::workspace();
    // A dependency of one member on the other, by its directory, so a dependency line is compared beside the members.
    let manifest = root.path().join("inside/Cargo.toml");
    let mut text = std::fs::read_to_string(&manifest).expect("the member's manifest");
    text.push_str("\n[dependencies]\noutside = { path = \"../outside\" }\n");
    std::fs::write(&manifest, text).expect("the member's manifest");
    // Cargo, run with its environment cleared and an empty `CARGO_HOME`: the tools it needs to be found are all it is given.
    let home = tempfile::tempdir().expect("a directory");
    let canonical = std::fs::canonicalize(root.path()).expect("the workspace is there");
    let (found_differences, said) =
        cargo_differences("workspace", &canonical, &home.path().to_string_lossy());
    assert!(said > 0, "cargo says nothing of the workspace's manifests");
    assert!(found_differences.is_empty(), "{found_differences:#?}");
}
