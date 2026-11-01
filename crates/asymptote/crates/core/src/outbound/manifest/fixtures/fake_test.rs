//! The `manifest` port's fake runs the port's contract.

use super::super::contract::check;
use super::ManifestFake;
use crate::domain::manifest::types::ProductRoot;

#[test]
fn succeeds() {
    // Absolute places on every platform: the contract holds a missing product's start to be one.
    let place = |name: &str| std::env::temp_dir().join(name).display().to_string();
    check(
        &ManifestFake::of(&place("fake"), &["fmt", "lint"]),
        &ProductRoot(place("fake").into()),
        &ProductRoot(place("elsewhere").into()),
        &ProductRoot(place("blank").into()),
    );
}
