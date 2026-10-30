//! The `manifest` port's fake runs the port's contract.

use super::super::contract::check;
use super::ManifestFake;

#[test]
fn succeeds() {
    check(&ManifestFake::default());
}
