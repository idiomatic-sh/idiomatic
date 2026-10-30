//! The `manifest` port's fake runs the port's contract.

use super::super::contract::check;
use super::ManifestFake;
use crate::domain::manifest::types::ProductRoot;

#[test]
fn succeeds() {
    check(
        &ManifestFake::of("/fake", &["fmt", "lint"]),
        &ProductRoot("/fake".into()),
        &ProductRoot("/elsewhere".into()),
        &ProductRoot("/blank".into()),
    );
}
